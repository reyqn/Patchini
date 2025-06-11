use std::cmp::min;
use std::collections::HashSet;
use std::fs;
use std::fs::{metadata, File};
use std::io::{BufRead, BufReader, Read, Seek, Write};
use std::path::Path;
use tar::{Archive, Builder, Entry, EntryType};
use walkdir::WalkDir;
use winsafe::gui::Edit;
use winsafe::prelude::{GuiWindow};
use winsafe::{msg, WString};
use zstd::zstd_safe::{CParameter};
use zstd::Decoder;

const CHUNK_SIZE: usize = 0x77777777;

fn create_path(path: &str, root: &str) -> Result<(), String> {
    if let Some(x) = path.rfind(std::path::MAIN_SEPARATOR_STR) {
        fs::create_dir_all(Path::join(root.as_ref(), &path[..x])).map_err(|_| format!("Couldn't create path {path} with root {root}"))?;
    }
    Ok(())
}

fn log_info(log: &Edit, text: &str) {
    let i = log.text().unwrap().len();
    log.set_selection(i as i32, i as i32);
    unsafe {
        log.hwnd().SendMessage(msg::em::ReplaceSel {
            can_be_undone: false, replacement_text: WString::from_str(format!("{text}\r\n"))
        })
    }
}

pub(crate) fn create_patch(old_file: String, new_file: String, lvl: i32, log: &Edit) -> Result<(), String> {
    if !metadata(&old_file).map_or(false, |x| x.is_dir()) { return Err("Old path doesn't exist or is not a directory".to_string()) };
    if !metadata(&new_file).map_or(false, |x| x.is_dir()) { return Err("New path doesn't exist or is not a directory".to_string()) };
    log.set_text("");

    let old_set = walk_dir(&old_file)?;
    let new_set = walk_dir(&new_file)?;

    let temp_dir = "patch";
    fs::create_dir_all(temp_dir).map_err(|_| "Couldn't create patch dir")?;

    log_info(log, "Compiling removed files");
    let mut rm_file = File::create(Path::join(temp_dir.as_ref(),"rm_files.txt")).map_err(|_| "Couldn't create rm_files.txt")?;
    old_set.difference(&new_set).try_for_each(|x| writeln!(rm_file, "{}", x).map_err(|_| "Couldn't write into rm_files.txt"))?;

    log_info(log, "Compiling added files");
    let new_files_path = Path::join(temp_dir.as_ref(), "new_files").to_str().ok_or("to_str failed for new_files_path")?.to_string();
    fs::create_dir_all(&new_files_path).map_err(|_| "Couldn't create new_files dir")?;
    new_set.difference(&old_set).try_for_each(|x| {
        create_path(x, &new_files_path)?;
        log_info(log, format!("adding file {x}").as_ref());
        match fs::copy(Path::join(new_file.as_ref(), x), Path::join(new_files_path.as_ref(), x)) {
            Ok(_) => {Ok(())}
            Err(_) => {Err(format!("Couldn't copy {x}"))}
        }
    })?;

    log_info(log, format!("Compiling changed files, compression level: {lvl}").as_ref());
    let diff_files_path = Path::join(temp_dir.as_ref(), "diff_files").to_str().ok_or("to_str failed for diff_files_path")?.to_string();
    fs::create_dir_all(&diff_files_path).map_err(|_| "Couldn't create diff_files dir")?;
    old_set.intersection(&new_set).try_for_each(|x| {
        let old_path = Path::join(old_file.as_ref(), x);
        let new_path = Path::join(new_file.as_ref(), x);

        log_info(log, format!("diffing file {x}").as_ref());
        let mut old = File::open(&old_path).map_err(|_| format!("Couldn't open old file {x}"))?;
        let mut new = File::open(&new_path).map_err(|_| format!("Couldn't open new file {x}"))?;
        let old_size = old.metadata().map_err(|_| format!("Couldn't get metadata for file {x}"))?.len();
        let mut i = 0;
        loop {
            i += 1;
            let mut old_data = Vec::with_capacity(min(old_size as usize, CHUNK_SIZE));
            let mut new_data = Vec::with_capacity(min(old_size as usize, CHUNK_SIZE));
            let n = Read::by_ref(&mut old).take(CHUNK_SIZE as u64).read_to_end(&mut old_data).map_err(|_| format!("Couldn't read old file {x}"))?;
            if old.stream_position().map_err(|_| format!("Couldn't get old file {x} stream position"))?.eq(&old_size) {
                Read::by_ref(&mut new).read_to_end(&mut new_data).map_err(|_| format!("Couldn't read new file {x}"))?;
            } else {
                Read::by_ref(&mut new).take(CHUNK_SIZE as u64).read_to_end(&mut new_data).map_err(|_| format!("Couldn't read new file {x} to end"))?;
            }
            if n == 0 { break; }
            if old_data.eq(&new_data) {
                continue;
            }
            let patch_data = create(old_data, new_data, lvl)?;
            let patch_file = Path::join(diff_files_path.as_ref(), x.to_string() + format!(".zspatch{i:0>3}").as_ref());
            create_path(x, &diff_files_path)?;
            fs::write(patch_file, patch_data).map_err(|_| format!("Couldn't write .zspatch file {x}"))?;
            if n < CHUNK_SIZE { break; }
        }
        Ok::<(), String>(())
    })?;

    log_info(log, "Generating patch file");
    let compressed_file = File::create("patch.patchini").map_err(|_| "Couldn't write create patchini file")?;
    let mut result = zstd::Encoder::new(compressed_file, 1).map_err(|_| "Couldn't create zstd encoder")?;
    {
        let mut archive = Builder::new(&mut result);
        WalkDir::new(temp_dir)
            .sort_by_file_name()
            .into_iter()
            .filter_map(|e| e.ok())
            .try_for_each(|x| {
                let appended_path = x.path();
                if appended_path.is_file() {
                    let mut appended_file = File::open(appended_path).map_err(|_| format!("Couldn't read tar file {}", appended_path.display()))?;
                    archive.append_file(appended_path.strip_prefix(temp_dir).map_err(|_| format!("Couldn't strip prefix {temp_dir}"))?, &mut appended_file).map_err(|_| format!("Couldn't append {} to tape", appended_path.display()))?
                }
                Ok::<(), String>(())
            })?

    }
    result.finish().map_err(|_| "Couldn't compress taped file")?;
    fs::remove_dir_all("patch").map_err(|_| "Couldn't cleanup")?;

    log_info(log, "Done");

    Ok(())
}

pub(crate) fn apply_patch(path: String, patch: String, log: &Edit) -> Result<(), String> {
    if !metadata(&path).map_or(false, |x| x.is_dir()) { return Err("Path to update doesn't exist or is not a directory".to_string()) };
    if !metadata(&patch).map_or(false, |x| x.is_file()) { return Err("Patch file doesn't exist".to_string()) };
    log.set_text("");
    std::env::set_current_dir(&path).map_err(|_| format!("Couldn't set current dir to {path}"))?;
    let mut patch_error = false;

    let backup_dir = "backup";
    fs::create_dir_all(backup_dir).map_err(|_| r"Couln't create backup dir")?;
    let mut last_file_name = "".to_string();
    let mut current_file = Option::<File>::None;

    let patch_file = File::open(patch).map_err(|_| r"Couldn't open patch file")?;
    let result = zstd::Decoder::new(patch_file).map_err(|_| "Couldn't create zstd decoder")?;
    {
        let mut a = Archive::new(result);
        for file in a.entries().map_err(|_| "Couldn't list tape entries")? {
            let mut file = file.map_err(|_| "Couldn't read tape entry")?;

            if file.header().entry_type() == EntryType::Directory {
                continue
            }

            let split: Vec<String> = file
                .path().map_err(|_| "Couldn't get path from tar file")?
                .to_str().ok_or("to_str failed for new_files_path")?
                .replace('/', std::path::MAIN_SEPARATOR_STR)
                .splitn(2, std::path::MAIN_SEPARATOR_STR)
                .map(String::from).collect();

            match split[0].as_str() {
                "new_files" => {
                    let added_file = split[1].as_str();
                    log_info(log, format!("adding {added_file}").as_ref());
                    add_file(&path, added_file, file)?;
                },
                "diff_files" => {
                    let diff_files_path = Path::join(backup_dir.as_ref(), "diff_files").to_str().ok_or("to_str failed for diff_files_path")?.to_string();
                    fs::create_dir_all(&diff_files_path).map_err(|_| "Couldn't create diff_files backup dir")?;
                    let ext = ".zspatch";
                    let ext_pos = split[1].rfind(ext).ok_or(format!("file {} doesn't contain extension", split[1]))?;
                    let new_file_name = split[1][..ext_pos].to_string();
                    if !last_file_name.eq(&new_file_name) {
                        move_file(&new_file_name, &diff_files_path)?;
                        if let Some(old_file) = current_file {
                            log_info(log, format!("no more patch data for {last_file_name}, copying from old file").as_ref());
                            let mut new_file = fs::OpenOptions::new().create(true).append(true).open(&last_file_name).map_err(|_| format!("Couldn't open {last_file_name} in write mode"))?;
                            std::io::copy(&mut &old_file, &mut new_file).map_err(|_| format!("Couldn't copy data from {last_file_name}"))?;
                        }
                        current_file = Some(File::open(Path::join(diff_files_path.as_ref(), &new_file_name)).map_err(|_| format!("Couldn't open {new_file_name} in backup dir"))?);
                        last_file_name = new_file_name.clone();
                    }
                    let mut old_file = current_file.as_ref().ok_or(format!("Couldn't get current file {new_file_name}"))?;

                    let mut patch_data = Vec::with_capacity(file.size() as usize);
                    let i = split[1][ext_pos+ext.len()..].parse::<u64>().map_err(|_| format!("Couldn't parse .zspatch number for {}", split[1]))?;
                    let mut old_data = Vec::with_capacity(min(old_file.metadata().map_err(|_| format!("Couldn't get metadata for file {new_file_name}"))?.len() as usize, CHUNK_SIZE));
                    let mut new_file = fs::OpenOptions::new().create(true).append(true).open(&new_file_name).map_err(|_| format!("Couldn't open {new_file_name} in write mode"))?;

                    let missing_chunks = i - 1 - old_file.stream_position().map_err(|_| format!("Couldn't get stream position for {new_file_name}"))? / (CHUNK_SIZE as u64);
                    if missing_chunks > 0 {
                        log_info(log, format!("no part until {i} for {new_file_name}, copying {missing_chunks} chunks as is").as_ref());
                        let mut take = Read::by_ref(&mut old_file).take(missing_chunks * CHUNK_SIZE as u64);
                        std::io::copy(&mut take, &mut new_file).map_err(|_| format!("Couldn't copy data from {new_file_name}"))?;
                    }

                    log_info(log, format!("applying diff {new_file_name} part {i}").as_ref());
                    Read::by_ref(&mut old_file).take(CHUNK_SIZE as u64).read_to_end(&mut old_data).map_err(|_| format!("Couldn't read {CHUNK_SIZE} for {new_file_name}"))?;
                    file.read_to_end(&mut patch_data).map_err(|_| format!("Couldn't read .zspatch{i} for {new_file_name}"))?;
                    match apply(old_data, patch_data) {
                        Ok(result) => {
                            new_file.write_all(&result).map_err(|_| format!("Couldn't write into {new_file_name}"))?;
                        }
                        Err(_) => {
                            patch_error = true;
                            log_info(log, &format!("Error while applying patch for {new_file_name}"))
                        }
                    }
                },
                "rm_files.txt" => {
                    log_info(log, "Removing files");
                    fs::create_dir_all("backup/rm_files").map_err(|_| "Couldn't create rm_files backup dir")?;
                    let reader = BufReader::new(file);
                    for line in reader.lines() {
                        let rem_file = line.map_err(|_| "Couldn't read line in rm_files.exe")?;
                        if move_file(&rem_file, "backup/rm_files").is_err() {
                            log_info(log, &format!("Couldn't remove {rem_file}"))
                        };
                    }
                }
                _ => {
                    return Err(format!("Unknown file in patch: {}", split[1]));
                }
            }
                
        }
    }

    log_info(log, "Done");
    let mut log_file = File::create("backup/logs.txt").map_err(|_| "Couldn't create logs.txt")?;
    log_file.write_all(log.text().unwrap().as_bytes()).map_err(|_| "Couldn't write logs.txt")?;
    if patch_error {
        return Err("Error(s) occurred while applying patch, check logs in backup dir for more info".to_string())
    }
    Ok(())
}

fn move_file(file: &String, new_dir: &str) -> Result<(), String> {
    create_path(file, new_dir)?;
    fs::rename(file, Path::join(new_dir.as_ref(), file)).map_err(|_| format!("Couldn't move {file} to {new_dir}"))?;
    Ok(())
}

fn add_file(path: &String, file: &str, mut entry: Entry<Decoder<BufReader<File>>>) -> Result<(), String> {
    let mut added_files = File::create("backup/added_files.txt").map_err(|_| "Couldn't create added_files.txt")?;
    writeln!(added_files, "{}", file).map_err(|_| "Couldn't write into added_files.txt")?;
    create_path(&file.replace('/', std::path::MAIN_SEPARATOR_STR), path)?;
    let mut test = File::create(Path::join(path.as_ref(), file)).map_err(|_| format!("Couldn't create {file} in {path}"))?;
    std::io::copy(&mut entry, &mut test).map_err(|_| format!("Couldn't extract {file} to {path}"))?;
    Ok(())
}

fn walk_dir(dir: &String) -> Result<HashSet<String>, String> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && !e.path().to_str().unwrap_or(r".patchiniored").contains(r".patchiniored"))
        .map(|x| match x.path().strip_prefix(dir) {
            Ok(o) => { Ok(o.to_str().ok_or(format!("to_str returned None for {}", o.display()))?.to_string()) }
            Err(_) => { Err(format!("Couldn't strip prefix {dir}")) }
        })
        .collect()
}

fn apply(old_data: Vec<u8>, patch_data: Vec<u8>) -> Result<Vec<u8>, ()> {
    let mut dict = zstd::zstd_safe::DCtx::create();
    let frame_content_size = zstd::zstd_safe::get_frame_content_size(&patch_data).map_err(|_| ())?.ok_or(())?;
    let mut new_data = Vec::with_capacity(frame_content_size as usize);

    dict.decompress_using_dict(&mut new_data, &patch_data, &old_data).map_err(|_| ())?;

    Ok(new_data)
}

fn create(old_data: Vec<u8>, new_data: Vec<u8>, lvl: i32) -> Result<Vec<u8>, String> {
    let high_bit = fio_high_bit64(old_data.len());
    let window_log = (high_bit+1).clamp(10, 31);

    let mut dict = zstd::zstd_safe::CCtx::create();
    dict.set_parameter(CParameter::CompressionLevel(lvl)).map_err(|_| "Couldn't set compression level")?;
    dict.set_parameter(CParameter::WindowLog(window_log)).map_err(|_| format!("Couldn't set window log {window_log}"))?;
    dict.set_parameter(CParameter::EnableLongDistanceMatching(true)).map_err(|_| "Couldn't enable long distance matching")?;
    dict.ref_prefix(&old_data).map_err(|_| "Couldn't apply ref prefix")?;

    let compress_bound = zstd::zstd_safe::compress_bound(new_data.len());

    let mut patch_data = Vec::with_capacity(compress_bound);
    dict.compress2(&mut patch_data, &new_data).map_err(|_| "Couldn't create zspatch data")?;

    Ok(patch_data)
}

fn fio_high_bit64(mut x: usize) -> u32 {
    let mut count = 0;
    x >>= 1;
    while x != 0 {
        x >>= 1;
        count += 1;
    }
    count
}