use std::path::Path;

use windows::{
    core::w,
    Win32::{
        Foundation::{HANDLE, HWND},
        System::DataExchange::{
            CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW,
            SetClipboardData,
        },
        UI::{
            Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_TYPE, KEYBDINPUT},
            WindowsAndMessaging::BringWindowToTop,
        },
    },
};

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
    ];
    unsafe { SendInput(&paste_cmd, std::mem::size_of::<INPUT>() as _) };
}

fn main() {
    println!("Hello, world!");
    paste_png(
        None,
        &std::path::PathBuf::from(r"C:\Users\Awsom\Downloads\Acheron_Sticker_01.png"),
    )
}
