use crate::ids;
use crate::patch::create_patch;
use std::time::Instant;
use winsafe::co::SW;
use winsafe::{self as w, co, gui, prelude::*, AnyResult, HWND, POINT};

#[derive(Clone)]
pub struct CreateTab {
    wnd: gui::WindowControl,
    _label_old: gui::Label,
    edit_old: gui::Edit,
    btn_old: gui::Button,
    _label_new: gui::Label,
    edit_new: gui::Edit,
    btn_new: gui::Button,
    track_lvl: gui::Trackbar,
    btn_create: gui::Button,
    edit_log: gui::Edit
}

impl AsRef<gui::WindowControl> for CreateTab { // we must implement AsRef so this window can be used as a tab
    fn as_ref(&self) -> &gui::WindowControl {
        &self.wnd
    }
}

impl CreateTab {
    pub fn new(parent: &impl GuiParent) -> Self {
        let dont_move = (gui::Horz::None, gui::Vert::None);

        let wnd = gui::WindowControl::new_dlg(parent, ids::DLG_CREATE, POINT::new(0, 0), dont_move, None);

        let _label_old = gui::Label::new_dlg(&wnd, ids::LBL_OLD, dont_move);
        let edit_old = gui::Edit::new_dlg(&wnd, ids::TXT_OLD, dont_move);
        let btn_old  = gui::Button::new_dlg(&wnd, ids::BTN_OLD, dont_move);
        let _label_new = gui::Label::new_dlg(&wnd, ids::LBL_NEW, dont_move);
        let edit_new = gui::Edit::new_dlg(&wnd, ids::TXT_NEW, dont_move);
        let btn_new  = gui::Button::new_dlg(&wnd, ids::BTN_NEW, dont_move);
        let _label_lvl = gui::Edit::new_dlg(&wnd, ids::TXT_LVL, dont_move);
        let track_lvl  = gui::Trackbar::new_dlg(&wnd, ids::TRACK_LVL, dont_move);
        let btn_create  = gui::Button::new_dlg(&wnd, ids::BTN_CREATE, dont_move);
        let edit_log = gui::Edit::new_dlg(&wnd, ids::TXT_CREATE, dont_move);

        let new_self = Self { wnd, _label_old, edit_old, btn_old, _label_new, edit_new, btn_new, track_lvl, btn_create, edit_log };
        new_self.events();
        new_self
    }

    fn switch_view(&self, show_logs: bool) {
        for x in ids::LBL_OLD..ids::BTN_CREATE {
            self.wnd.hwnd().GetDlgItem(x).unwrap().ShowWindow(if show_logs {SW::HIDE} else {SW::SHOW});
        }
        self.edit_log.hwnd().ShowWindow(if show_logs {SW::SHOW} else {SW::HIDE});
    }

    fn events(&self) {
        let self2 = self.clone();
        self.wnd.on().wm_init_dialog(move |_| {
            self2.track_lvl.set_range(1,22);
            self2.switch_view(false);
            Ok(true)
        });

        let self2 = self.clone();
        self.btn_old.on().bn_clicked(move || {
            let fileo = w::CoCreateInstance::<w::IFileOpenDialog>(
                &co::CLSID::FileOpenDialog,
                None,
                co::CLSCTX::INPROC_SERVER,
            )?;

            fileo.SetOptions(
                fileo.GetOptions()?
                    | co::FOS::FORCEFILESYSTEM
                    | co::FOS::PICKFOLDERS
            )?;

            if fileo.Show(self2.wnd.hwnd())? {
                let text = fileo.GetResult()?.GetDisplayName(co::SIGDN::FILESYSPATH)?;
                self2.edit_old.set_text(text.as_str());
            }

            Ok(())
        });

        let self2 = self.clone();
        self.btn_new.on().bn_clicked(move || {
            let fileo = w::CoCreateInstance::<w::IFileOpenDialog>(
                &co::CLSID::FileOpenDialog,
                None,
                co::CLSCTX::INPROC_SERVER,
            )?;

            fileo.SetOptions(
                fileo.GetOptions()?
                    | co::FOS::FORCEFILESYSTEM
                    | co::FOS::PICKFOLDERS
            )?;

            if fileo.Show(self2.wnd.hwnd())? {
                let text = fileo.GetResult()?.GetDisplayName(co::SIGDN::FILESYSPATH)?;
                self2.edit_new.set_text(text.as_str());
            }

            Ok(())
        });

        let self2 = self.clone();
        self2.btn_create.clone().on().bn_clicked({
            move || -> AnyResult<()> {
                std::thread::spawn({
                    *crate::main_window::EPOCH.lock().unwrap() = Some(Instant::now());
                    self2.switch_view(true);
                    self2.btn_create.hwnd().EnableWindow(false);
                    let old_path = self2.edit_old.text().to_string();
                    let new_path = self2.edit_new.text().to_string();
                    let lvl = self2.track_lvl.pos();
                    let self3 = self2.clone();
                    move || {
                        match create_patch(old_path, new_path, lvl, &self3.edit_log) {
                            Ok(_) => {
                                *crate::main_window::EPOCH.lock().unwrap() = None;
                                HWND::NULL.MessageBox(
                                    "Patch created successfully", "Success", co::MB::ICONINFORMATION).unwrap()
                            }
                            Err(str) => {
                                *crate::main_window::EPOCH.lock().unwrap() = None;
                                HWND::NULL.MessageBox(
                                    &str, "Error", co::MB::ICONWARNING).unwrap()
                            }
                        };

                        self3.btn_create.hwnd().EnableWindow(true);
                        self3.switch_view(false);
                    }
                });

                Ok(())
            }
        });
    }
}

