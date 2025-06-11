use crate::ids;
use crate::patch::apply_patch;
use std::time::Instant;
use winsafe::co::SW;
use winsafe::{self as w, co, gui, prelude::*, HWND};

#[derive(Clone)]
pub struct ApplyTab {
    wnd: gui::WindowControl,
    _label_path: gui::Label,
    edit_path: gui::Edit,
    btn_path: gui::Button,
    _label_patch: gui::Label,
    edit_patch: gui::Edit,
    btn_patch: gui::Button,
    btn_apply: gui::Button,
    apply_log: gui::Edit
}

impl AsRef<gui::WindowControl> for ApplyTab {
    fn as_ref(&self) -> &gui::WindowControl {
        &self.wnd
    }
}

impl ApplyTab {
    pub fn new(parent: &(impl GuiParent + 'static)) -> Self {
        let dont_move = (gui::Horz::None, gui::Vert::None);

        let wnd = gui::WindowControl::new_dlg(parent, ids::DLG_APPLY, (0, 0), dont_move, None);

        let _label_path = gui::Label::new_dlg(&wnd, ids::LBL_PATH, dont_move);
        let edit_path = gui::Edit::new_dlg(&wnd, ids::TXT_PATH, dont_move);
        let btn_path  = gui::Button::new_dlg(&wnd, ids::BTN_PATH, dont_move);
        let _label_patch = gui::Label::new_dlg(&wnd, ids::LBL_PATCH, dont_move);
        let edit_patch = gui::Edit::new_dlg(&wnd, ids::TXT_PATCH, dont_move);
        let btn_patch  = gui::Button::new_dlg(&wnd, ids::BTN_PATCH, dont_move);
        let btn_apply  = gui::Button::new_dlg(&wnd, ids::BTN_APPLY, dont_move);
        let apply_log = gui::Edit::new_dlg(&wnd, ids::TXT_APPLY, dont_move);

        let new_self = Self { wnd, _label_path, edit_path, btn_path, _label_patch, edit_patch, btn_patch, btn_apply, apply_log };
        new_self.events();
        new_self
    }

    fn switch_view(&self, show_logs: bool) {
        for x in ids::LBL_PATH..ids::BTN_APPLY {
            self.wnd.hwnd().GetDlgItem(x).unwrap().ShowWindow(if show_logs {SW::HIDE} else {SW::SHOW});
        }
        self.apply_log.hwnd().ShowWindow(if show_logs {SW::SHOW} else {SW::HIDE});
    }

    fn events(&self) {
        let self2 = self.clone();
        self.wnd.on().wm_init_dialog(move |_| {
            self2.switch_view(false);
            Ok(true)
        });

        let self2 = self.clone();
        self.btn_path.on().bn_clicked(move || {
            let fileo = w::CoCreateInstance::<w::IFileOpenDialog>(
                &co::CLSID::FileOpenDialog,
                None::<&w::IUnknown>,
                co::CLSCTX::INPROC_SERVER,
            )?;

            fileo.SetOptions(
                fileo.GetOptions()?
                    | co::FOS::FORCEFILESYSTEM
                    | co::FOS::PICKFOLDERS
            )?;

            if fileo.Show(self2.wnd.hwnd())? {
                let text = fileo.GetResult()?.GetDisplayName(co::SIGDN::FILESYSPATH)?;
                self2.edit_path.set_text(text.as_str());
            }

            Ok(())
        });

        let self2 = self.clone();
        self.btn_patch.on().bn_clicked(move || {
            let fileo = w::CoCreateInstance::<w::IFileOpenDialog>(
                &co::CLSID::FileOpenDialog,
                None::<&w::IUnknown>,
                co::CLSCTX::INPROC_SERVER,
            )?;

            fileo.SetOptions(
                fileo.GetOptions()?
                    | co::FOS::FORCEFILESYSTEM
                    | co::FOS::FILEMUSTEXIST
            )?;

            fileo.SetFileTypes(&[
                ("Patch files", "*.patchini"),
                ("All files", "*.*"),
            ])?;

            if fileo.Show(self2.wnd.hwnd())? {
                let text = fileo.GetResult()?.GetDisplayName(co::SIGDN::FILESYSPATH)?;
                self2.edit_patch.set_text(text.as_str());
            }

            Ok(())
        });

        let self2 = self.clone();
        self2.btn_apply.clone().on().bn_clicked({
            move || -> w::AnyResult<()> {
                std::thread::spawn({
                    *crate::main_window::EPOCH.lock().unwrap() = Some(Instant::now());
                    self2.switch_view(true);
                    self2.btn_apply.hwnd().EnableWindow(false);
                    let old_path = self2.edit_path.text().unwrap().to_string();
                    let new_path = self2.edit_patch.text().unwrap().to_string();
                    let self3 = self2.clone();
                    move || {
                        match apply_patch(old_path, new_path, &self3.apply_log) {
                            Ok(_) => {
                                *crate::main_window::EPOCH.lock().unwrap() = None;
                                HWND::NULL.MessageBox(
                                    "Patch applied successfully", "Success", co::MB::ICONINFORMATION).unwrap()
                            }
                            Err(str) => {
                                *crate::main_window::EPOCH.lock().unwrap() = None;
                                HWND::NULL.MessageBox(
                                    &str, "Error", co::MB::ICONWARNING).unwrap()
                            }
                        };
                        self3.btn_apply.hwnd().EnableWindow(true);
                        self3.switch_view(false);
                    }
                });

                Ok(())
            }
        });
    }
}