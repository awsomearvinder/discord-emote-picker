use std::{
    hash::{Hash, Hasher},
    mem::MaybeUninit,
    path::Path,
    pin::Pin,
};

use futures::{self, Stream};
use fuzzy_matcher::FuzzyMatcher;
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
    LoadedEntries(Vec<String>),
}

struct EmotePicker {
    emote_text: String,
    win: Option<window::Id>,
    entries: Vec<String>,
    emote_index: i32,
    winapi_events: async_channel::Receiver<Messages>,
    text_id: iced::widget::text_input::Id,
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
                entries: vec![],
                emote_text: String::new(),
                emote_index: 0,
                win: None,
                winapi_events: flags.0,
                text_id: text_input::Id::unique(),
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
        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
        match msg {
            Messages::EmoteInput(text) => {
                self.emote_index = 0;
                self.emote_text = text.clone();
                iced::Task::perform(
                    async move {
                        let mut dir = tokio::fs::read_dir(r"C:\Users\Awsom\Documents\Emotes")
                            .await
                            .unwrap();

                        let mut contents = vec![];
                        while let Ok(Some(next_entry)) = dir.next_entry().await {
                            contents.push(next_entry.path().to_string_lossy().to_string());
                        }

                        contents
                            .sort_unstable_by_key(|i| matcher.fuzzy_match(i, &text).unwrap_or(-1));
                        contents
                    },
                    Messages::LoadedEntries,
                )
            }
            Messages::EmoteSelect => todo!(),
            Messages::EmotePickerToggle => match self.win {
                Some(t) => {
                    self.win = None;
                    iced::window::close(t)
                }
                None => iced::window::open({
                    let mut settings = Settings::default();
                    settings.decorations = false;
                    settings.position = Position::Centered;
                    settings.level = iced::window::Level::AlwaysOnTop;
                    settings
                })
                .map(Messages::WindowOpen),
            },
            Messages::WindowOpen(id) => {
                self.win = Some(id);
                self.emote_index = 0;
                iced::window::gain_focus(id)
                    .chain(iced::widget::text_input::focus(self.text_id.clone()))
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
            Messages::LoadedEntries(entries) => {
                self.entries = entries;
                iced::Task::none()
            }
            _ => iced::Task::none(),
        }
    }

    fn view(&self, _: window::Id) -> iced::Element<'_, Messages, iced::Theme, iced::Renderer> {
        let input = text_input("Emote", &self.emote_text)
            .on_input(Messages::EmoteInput)
            .on_submit(Messages::EmoteSelect)
            .id(self.text_id.clone())
            .padding(10)
            .size(25);

        let options = self
            .entries
            .iter()
            .rev()
            .map(|entry| {
                container(iced::widget::row![
                    container(iced::widget::image(entry).height(Pixels(35.0)))
                        .center_x(40)
                        .center_y(40),
                    iced::widget::text(entry).size(Pixels(25.0))
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
