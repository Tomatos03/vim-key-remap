#![windows_subsystem = "windows"]

use std::{collections::HashSet, sync::{atomic::Ordering, mpsc::{self, Receiver, Sender}, Arc, Mutex}, thread};

use image::GenericImageView;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, MenuId},
    TrayIconBuilder, Icon,
};
use winit::{application::ApplicationHandler};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;
use std::sync::atomic::AtomicBool;

static IS_EXIT: AtomicBool = AtomicBool::new(false);
static EXIT_SUCCESS: AtomicBool = AtomicBool::new(false);

struct App {
    exit_id: MenuId,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _event: WindowEvent,
    ) {}

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);

        // 检查菜单事件
        if let Ok(menu_event) = MenuEvent::receiver().try_recv() {
            if menu_event.id == self.exit_id {
                println!("接收到退出信号，程序即将退出...");

                IS_EXIT.store(true, Ordering::Release);

                // 等待intercept_thread完成清理
                while !EXIT_SUCCESS.load(Ordering::Acquire) {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }

                event_loop.exit();
            }
        }
    }
}

fn main() {
    // 创建图标
    let icon = {
        let icon_data = include_bytes!("../res/icon.ico");
        match image::load_from_memory(icon_data) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (width, height) = img.dimensions();
                Icon::from_rgba(rgba.into_raw(), width, height).expect("Failed to create icon")
            }
            Err(e) => {
                println!("警告：图标加载失败 ({}), 使用默认图标", e);
                let rgba = vec![255u8; 16 * 16 * 4]; // 16x16 白色图标
                Icon::from_rgba(rgba, 16, 16).expect("Failed to create default icon")
            }
        }
    };

    // 创建菜单
    let exit_id = MenuId::new("exit");
    let menu = Menu::new();
    let menu_item = MenuItem::with_id(exit_id.clone(), "退出程序", true, None);
    menu.append(&menu_item).expect("Failed to create menu item");

    // 创建系统托盘
    let _tray_icon = TrayIconBuilder::new()
        .with_tooltip("key_remap running...")
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .build()
        .expect("Failed to create tray icon");

    // 启动按键映射处理线程
    thread::spawn(intercept_thread);

    // 创建事件循环和应用程序处理器
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app = App { exit_id };
    event_loop.run_app(&mut app).expect("Event loop failed");
}

fn intercept_thread() {
    use interception::{Interception, Stroke, KeyState, ScanCode, Filter, KeyFilter, is_keyboard};
    use std::time::{Duration};

    const THRESHOLD: Duration = Duration::from_millis(80);

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum State { Idle, Pressed, Holding }


    let ctx = match Interception::new() {
        Some(ctx) => ctx,
        None => {
            eprintln!("创建 interception context 失败");
            std::process::exit(1);
        }
    };
    ctx.set_filter(is_keyboard, Filter::KeyFilter(KeyFilter::UP | KeyFilter::DOWN));

    let (timer_tx, timer_rx): (Sender<i32>, Receiver<i32>) = mpsc::channel();

    let state_arc = Arc::new(Mutex::new(State::Idle));

    let mut buf = [Stroke::Keyboard {
        code: ScanCode::A,
        state: KeyState::DOWN,
        information: 0,
    }];

    let mut set: HashSet<(ScanCode, i32)> = HashSet::new();
    while !IS_EXIT.load(Ordering::Acquire) {
        let state_clone = state_arc.clone();

        if let Ok(device) = timer_rx.try_recv() {
            let state = state_clone.lock().unwrap_or_else(|p| p.into_inner());
            if *state == State::Holding {
                ctx.send(device, &[lctrl(KeyState::DOWN)]);
                set.insert((ScanCode::LeftControl, device));
            }
        }

        let device = ctx.wait_with_timeout(Duration::from_millis(1));
        if device == 0 { continue; }

        if ctx.receive(device, &mut buf) == 0 { continue; }

        let (code, key_state) = match buf[0] {
            Stroke::Keyboard { code, state, .. } => (code, state),
            _ => { ctx.send(device, &buf); continue; }
        };

        if code != ScanCode::CapsLock {
            ctx.send(device, &buf);
            continue;
        }

        let state = &mut *state_arc.lock().unwrap_or_else(|p| p.into_inner());
        match (key_state, *state) {
            (KeyState::DOWN, State::Idle) => {
                *state = State::Pressed;

                let tx_clone = timer_tx.clone();
                thread::spawn(move || {
                    thread::sleep(THRESHOLD);

                    match state_clone.try_lock() {
                        Ok(mut guard) => {
                            if *guard == State::Pressed {
                                *guard = State::Holding;
                                let _ = tx_clone.send(device);
                            }
                        }
                        Err(_) => {}
                    }
                });
            }
            (KeyState::DOWN, State::Pressed) => {
                ctx.send(device, &[esc(KeyState::DOWN)]);
                set.insert((ScanCode::Esc, device));
            }
            (KeyState::DOWN, State::Holding) => {
                ctx.send(device, &[lctrl(KeyState::DOWN)]);
                set.insert((ScanCode::LeftControl, device));
            }
            (KeyState::UP, State::Pressed) => {
                ctx.send(device, &[esc(KeyState::DOWN)]);
                ctx.send(device, &[esc(KeyState::UP)]);
                *state = State::Idle;
            }
            (KeyState::UP, State::Holding) => {
                ctx.send(device, &[lctrl(KeyState::UP)]);
                *state = State::Idle;
            }
            _ => {}
        }
    }

    for (code, device_id) in &set {
        ctx.send(*device_id, &[Stroke::Keyboard { code: *code, state: KeyState::UP, information: 0 }]);
    }

    EXIT_SUCCESS.store(true, Ordering::Release);

    fn esc(st: KeyState) -> Stroke {
        Stroke::Keyboard { code: ScanCode::Esc, state: st, information: 0 }
    }
    fn lctrl(st: KeyState) -> Stroke {
        Stroke::Keyboard { code: ScanCode::LeftControl, state: st, information: 0 }
    }
}
