use crate::apply_tab::ApplyTab;
use crate::create_tab::CreateTab;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;
use winsafe::prelude::{user_Hwnd, GuiParent, GuiWindow};
use winsafe::{co, gui, AnyResult};

#[derive(Clone)]
pub struct MainWindow {
    wnd: gui::WindowMain,
    status: gui::StatusBar
}

pub(crate) static EPOCH: LazyLock<Mutex<Option<Instant>>> = LazyLock::new(|| Mutex::new(None));

impl MainWindow {
    pub fn new() -> Self {
        let wnd = gui::WindowMain::new(
            gui::WindowMainOpts {
                title: "Patchini".to_owned(),
                class_icon: gui::Icon::Id(101),
                size: (380, 300),
                style: gui::WindowMainOpts::default().style | co::WS::MINIMIZEBOX,
                ..Default::default()
            },
        );

        let status  = gui::StatusBar::new(&wnd, &[
            gui::SbPart::Proportional(1)
        ]);

        let _ = gui::Tab::new(
            &wnd,
            gui::TabOpts {
                position: (10, 10),
                size: (360, 260),
                items: vec![
                    ("Apply".to_owned(), Box::new(ApplyTab::new(&wnd))),
                    ("Create".to_owned(), Box::new(CreateTab::new(&wnd))),
                ],
                ..Default::default()
            },
        );

        let new_self = Self { wnd, status };
        new_self.events();
        new_self
    }

    pub fn run(&self) -> AnyResult<i32> {
        self.wnd.run_main(None)
    }

    fn events(&self) {
        let self2 = self.clone();
        self.wnd.on().wm_create(move |_| {
            self2.wnd.hwnd().SetTimer(1, 1000, None).unwrap();
            Ok(0)
        });

        let self2 = self.clone();
        self2.wnd.on().wm_timer(1, move || update_status(&self2.status));
    }
}

fn update_status(status: &gui::StatusBar) -> AnyResult<()> {
    if let Some(x) = *EPOCH.lock().unwrap() {
        status.parts().get(0).set_text(&x.elapsed().as_secs().to_string());
    }
    Ok(())
}