use std::{
    hash::{Hash, Hasher},
    mem::MaybeUninit,
    path::Path,
    pin::Pin,
};

use futures::{self, Stream};
use iced::{
    advanced::{graphics::core::window, subscription::Recipe},
    widget::{container, scrollable, text::LineHeight, text_input},
    window::{Position, Settings},
    Background, Border, Color,
    Length::Fill,
    Pixels, Shadow,
};
use windows::{
    core::w,
    Win32::{
        Foundation::{HANDLE, HWND},
        System::DataExchange::{
            CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW,
            SetClipboardData,
        },
        UI::{
            Input::KeyboardAndMouse::{
                RegisterHotKey, SendInput, INPUT, INPUT_0, INPUT_TYPE, KEYBDINPUT, KEYEVENTF_KEYUP,
                MOD_CONTROL, MOD_NOREPEAT,
            },
            WindowsAndMessaging::{BringWindowToTop, GetMessageW, MSG, WM_HOTKEY},
        },
    },
};

#[derive(Debug, Clone)]
enum Messages {
    EmoteInput(String),
    EmoteSelect,
    EmotePickerToggle,
    WindowOpen(window::Id),
    Event(iced::Event),
}

struct EmotePicker {
    emote_text: String,
    win: Option<window::Id>,
    emote_index: i32,
    winapi_events: async_channel::Receiver<Messages>,
}

struct ExternalMessageStreamRecipe(Pin<Box<dyn Stream<Item = Messages> + Send>>);
impl Recipe for ExternalMessageStreamRecipe {
    type Output = Messages;

    fn hash(&self, state: &mut rustc_hash::FxHasher) {
        (&*self.0 as *const dyn Stream<Item = Messages>).hash(state);
    }

    fn stream(
        self: Box<Self>,
        _: iced::advanced::subscription::EventStream,
    ) -> iced::advanced::graphics::futures::BoxStream<Self::Output> {
        self.0
    }
}

impl EmotePicker {
    fn new(flags: (async_channel::Receiver<Messages>,)) -> (Self, iced::Task<Messages>) {
        (
            EmotePicker {
                emote_text: String::new(),
                emote_index: 0,
                win: None,
                winapi_events: flags.0,
            },
            iced::Task::none(),
        )
    }

    fn subscription(&self) -> iced::Subscription<Messages> {
        iced::Subscription::batch([
            iced::advanced::subscription::from_recipe(ExternalMessageStreamRecipe(Box::pin(
                self.winapi_events.clone(),
            )
                as Pin<Box<dyn Stream<Item = Messages> + Send>>)),
            iced::event::listen().map(Messages::Event),
        ])
    }

    fn title(&self, _: window::Id) -> String {
        String::from("Windows API")
    }

    fn theme(&self, _: window::Id) -> iced::Theme {
        iced::Theme::default()
    }

    fn update(&mut self, msg: Messages) -> iced::Task<Messages> {
        match msg {
            Messages::EmoteInput(text) => {
                self.emote_index = 0;
                self.emote_text = text;
                iced::Task::none()
            }
            Messages::EmoteSelect => todo!(),
            Messages::EmotePickerToggle => match self.win {
                Some(t) => {
                    self.win = None;
                    iced::window::close(t)
                }
                None => iced::window::open({
                    let mut settings = Settings::default();
                    settings.position = Position::Centered;
                    settings
                })
                .map(Messages::WindowOpen),
            },
            Messages::WindowOpen(id) => {
                self.win = Some(id);
                self.emote_index = 0;
                iced::Task::none()
            }
            Messages::Event(iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key:
                    iced::keyboard::Key::Named(
                        key @ (iced::keyboard::key::Named::ArrowDown
                        | iced::keyboard::key::Named::ArrowUp),
                    ),
                ..
            })) => {
                match key {
                    iced::keyboard::key::Named::ArrowDown => {
                        self.emote_index += 1;
                    }
                    iced::keyboard::key::Named::ArrowUp => {
                        self.emote_index -= 1;
                    }
                    _ => unreachable!(),
                }
                iced::Task::none()
            }
            Messages::Event(iced::Event::Window(iced::window::Event::Closed { .. })) => {
                self.win = None;
                iced::Task::none()
            }
            _ => iced::Task::none(),
        }
    }

    fn view(&self, _: window::Id) -> iced::Element<'_, Messages, iced::Theme, iced::Renderer> {
        let input = text_input("Emote", &self.emote_text)
            .on_input(Messages::EmoteInput)
            .on_submit(Messages::EmoteSelect)
            .padding(10)
            .size(25);

        let options = (0..10)
            .map(|_| {
                container(iced::widget::row![
                    iced::widget::text("test").size(Pixels(25.0))
                ])
                .width(Fill)
                .padding(10)
            })
            .enumerate()
            .map(|(i, item)| {
                if i == self.emote_index as usize {
                    item.style(|_| container::Style {
                        text_color: Default::default(),
                        background: Some(Background::Color(Color {
                            r: 0.2,
                            g: 0.,
                            b: 0.8,
                            a: 1.0,
                        })),
                        border: Border::default(),
                        shadow: Shadow::default(),
                    })
                } else {
                    item
                }
            });

        let options = iced::widget::column(options.map(Into::into));
        let options = scrollable(options);
        iced::widget::column![input, options,].into()
    }
}

// FUCKING WHAT did I just write...
fn paste_png(win: Option<HWND>, path: &Path) {
    let win = match win {
        Some(win) if !win.is_invalid() => unsafe {
            BringWindowToTop(win).unwrap();
            win
        },
        _ => HWND::default(),
    };
    unsafe { OpenClipboard(win).unwrap() };
    let fmt = unsafe { RegisterClipboardFormatW(w!("FileName")) };
    unsafe { EmptyClipboard().unwrap() };
    unsafe {
        SetClipboardData(
            fmt,
            HANDLE(path.as_os_str().to_str().unwrap().as_ptr() as _),
        )
        .unwrap()
    };
    unsafe { CloseClipboard().unwrap() };

    let paste_cmd = [
        INPUT {
            r#type: INPUT_TYPE(1), // keyboard input type
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0x11),
                    wScan: 0,
                    dwFlags:
                        windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS::default(),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_TYPE(1), // keyboard input type
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0x56),
                    wScan: 0,
                    dwFlags:
                        windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS::default(),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_TYPE(1), // keyboard input type
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0x11),
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_TYPE(1), // keyboard input type
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0x56),
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    unsafe { SendInput(&paste_cmd, std::mem::size_of::<INPUT>() as _) };
}

fn main() {
    std::thread::sleep_ms(1000);
    // paste_png(
    //     None,
    //     &std::path::PathBuf::from(r"C:\Users\Awsom\Downloads\Acheron_Sticker_01.png"),
    // )

    let (send, recv) = async_channel::bounded(1);
    std::thread::spawn(move || {
        unsafe { RegisterHotKey(HWND::default(), 0, MOD_CONTROL | MOD_NOREPEAT, 0xBA).unwrap() }

        let mut msg = MaybeUninit::uninit();
        while unsafe { GetMessageW(&mut msg as *mut _ as _, HWND::default(), 0, 0).into() } {
            let msg: MSG = unsafe { msg.assume_init() };
            if msg.message == WM_HOTKEY {
                futures::executor::block_on(send.send(Messages::EmotePickerToggle)).unwrap();
            }
        }
    });
    iced::daemon(EmotePicker::title, EmotePicker::update, EmotePicker::view)
        .subscription(EmotePicker::subscription)
        .theme(EmotePicker::theme)
        .run_with(move || EmotePicker::new((recv,)))
        .unwrap();
}
