use std::{
    env::args,
    ffi::CStr,
    fs::{read_dir, read_link, File, Metadata},
    io::{stdout, BufWriter, StdoutLock, Write},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
    time::UNIX_EPOCH,
};

use chrono::{DateTime, Datelike, Duration, Timelike};
use libc::{getgrgid, getpwuid};

fn main() {
    let args = args().skip(1);
    let mut path = "./".to_string();
    let mut is_long = false;
    let mut is_sort = true;
    let mut is_f_info = false;
    let mut is_all = false;
    let mut is_under = false;
    let mut is_dir_info = false;
    let mut is_time_sort = false;
    let mut is_size_sort = false;
    let mut is_h = false;
    let mut is_reverse = false;

    let mut under_quae = Vec::with_capacity(100);

    for arg in args {
        if arg.contains("-") {
            let temp = arg.chars().skip(1);
            for t in temp {
                match t {
                    'l' => is_long = true,
                    'f' => is_sort = false,
                    'F' => is_f_info = true,
                    'a' => is_all = true,
                    'R' => is_under = true,
                    'd' => is_dir_info = true,
                    't' => is_time_sort = true,
                    'h' => is_h = true,
                    's' => is_size_sort = true,
                    'r' => is_reverse = true,
                    _ => {}
                }
            }
        } else {
            path = arg
        }
    }

    under_quae.push(path);

    let get_permition_print = |metadata: &Metadata| {
        let mut permition = None;
        if is_long {
            permition = Some(format!("{:o}", metadata.permissions().mode()));
            let p_len = permition.clone().unwrap().len();
            permition = Some(
                if metadata.is_dir() {
                    "d".to_string()
                } else if metadata.is_symlink() {
                    "l".to_string()
                } else {
                    "-".to_string()
                } + (print_permition(&permition.unwrap()[p_len - 3..]) + " ").as_str(),
            );
        }
        permition
    };

    let get_h_print = |metadata: &Metadata| {
        if is_h {
            let mut len = metadata.len();
            let kmgtp = ["", "K", "M", "G", "T", "P"];
            let mut key = "";
            for d in kmgtp {
                if len >= 1024 {
                    len /= 1024;
                } else {
                    key = d;
                    break;
                }
            }
            len.to_string() + key + "\t"
        } else {
            metadata.len().to_string() + "\t"
        }
    };

    let get_date = |metadata: &Metadata| {
        let (secs, nsec) = match metadata.modified().unwrap().duration_since(UNIX_EPOCH) {
            Ok(dur) => (dur.as_secs() as i64, dur.subsec_nanos()),
            Err(e) => {
                let dur = e.duration();
                let (sec, nsec) = (dur.as_secs() as i64, dur.subsec_nanos());
                if nsec == 0 {
                    (-sec, 0)
                } else {
                    (-sec - 1, 1_000_000_000 - nsec)
                }
            }
        };
        let day = DateTime::from_timestamp(secs, nsec).unwrap() + Duration::hours(9);
        format!(
            "{}월 {}일 {:02}:{:02} ",
            day.month(),
            day.day(),
            day.hour(),
            day.minute()
        )
    };

    let get_file_name = |metadata: &Metadata, file_name: String, path: &Path| {
        let file_name = file_name
            + if is_f_info && metadata.is_dir() {
                "/"
            } else if is_f_info && (metadata.permissions().mode() & 0o111) == 0o111 {
                "*"
            } else {
                ""
            };
        let link = if metadata.is_symlink() {
            let link_path = Path::new(&path);
            match read_link(&link_path) {
                Ok(target) => format!(" -> {}", target.display()),
                Err(e) => {
                    format!(" -> {}", e)
                }
            }
        } else {
            "".to_string()
        };

        format!("{}{}", file_name, link)
    };
    let meup_print = |bw: &mut BufWriter<StdoutLock<'static>>| {
        if is_all {
            let meup = if is_reverse { ["..", "."] } else { [".", ".."] };
            for m in meup {
                let me = File::open(m);
                if let Ok(mefile) = me {
                    let metadata = mefile.metadata().unwrap();
                    if is_long {
                        let pw = unsafe { getpwuid(metadata.uid()) };
                        let gr = unsafe { getgrgid(metadata.gid()) };
                        let permition = get_permition_print(&metadata);
                        writeln!(
                            bw,
                            "{}{}{}{}{}{}{}",
                            permition.unwrap_or("".to_string()),
                            metadata.nlink().to_string() + "\t",
                            if pw.is_null() {
                                "unknown".to_string()
                            } else {
                                let user_name = unsafe {
                                    CStr::from_ptr((*pw).pw_name).to_string_lossy().into_owned()
                                };
                                user_name.to_string() + "\t"
                            },
                            if gr.is_null() {
                                "unknown".to_string()
                            } else {
                                let group_name = unsafe {
                                    CStr::from_ptr((*gr).gr_name).to_string_lossy().into_owned()
                                };
                                group_name.to_string() + "\t"
                            },
                            get_h_print(&metadata),
                            get_date(&metadata),
                            {
                                let path = Path::new(m);
                                get_file_name(&metadata, m.to_string(), path)
                            }
                        )
                        .unwrap();
                    } else {
                        let path = Path::new(m);
                        write!(bw, "{}\t", get_file_name(&metadata, m.to_string(), path)).unwrap();
                    }
                }
            }
        }
    };

    let mut bw = BufWriter::new(stdout().lock());
    let print_dir =
        |bw: &mut BufWriter<StdoutLock<'static>>, path: &Path, under_quae: &mut Vec<String>| {
            let dir = read_dir(path).unwrap();
            let mut entrys = dir
                .into_iter()
                .filter(|entry| entry.is_ok())
                .map(|entry| entry.unwrap())
                .collect::<Vec<_>>();
            if is_sort {
                entrys.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
            }

            if is_time_sort {
                entrys.sort_by(|a, b| {
                    a.metadata()
                        .unwrap()
                        .mtime()
                        .cmp(&b.metadata().unwrap().mtime())
                });
            }

            if is_size_sort {
                entrys.sort_by(|a, b| {
                    b.metadata()
                        .unwrap()
                        .len()
                        .cmp(&a.metadata().unwrap().len())
                });
            }
            if !is_reverse {
                meup_print(bw);
            }
            if is_under {
                writeln!(bw, "{}:", path.display()).unwrap();
            }
            for entry in entrys.iter() {
                if !is_all && entry.file_name().to_string_lossy().chars().next().unwrap() == '.' {
                    continue;
                }
                if let Ok(metadata) = entry.metadata() {
                    let permition = get_permition_print(&metadata);
                    if is_long {
                        let pw = unsafe { getpwuid(metadata.uid()) };
                        let gr = unsafe { getgrgid(metadata.gid()) };
                        writeln!(
                            bw,
                            "{}{}{}{}{}{}{}",
                            permition.unwrap_or("".to_string()),
                            metadata.nlink().to_string() + "\t",
                            if pw.is_null() {
                                "unknown".to_string()
                            } else {
                                let user_name = unsafe {
                                    CStr::from_ptr((*pw).pw_name).to_string_lossy().into_owned()
                                };
                                user_name.to_string() + "\t"
                            },
                            if gr.is_null() {
                                "unknown".to_string()
                            } else {
                                let group_name = unsafe {
                                    CStr::from_ptr((*gr).gr_name).to_string_lossy().into_owned()
                                };
                                group_name.to_string() + "\t"
                            },
                            get_h_print(&metadata),
                            get_date(&metadata),
                            {
                                let entry_path = entry.path();
                                let entry_path = Path::new(&entry_path);
                                get_file_name(
                                    &metadata,
                                    entry.file_name().to_str().unwrap().to_string(),
                                    entry_path,
                                )
                            }
                        )
                        .unwrap();
                    } else {
                        let entry_path = entry.path();
                        let entry_path = Path::new(&entry_path);
                        write!(
                            bw,
                            "{}\t",
                            get_file_name(
                                &metadata,
                                entry.file_name().to_str().unwrap().to_string(),
                                entry_path
                            )
                        )
                        .unwrap();
                    }
                }
            }
            if is_reverse {
                meup_print(bw);
            }

            writeln!(bw, "").unwrap();
            if is_under {
                for entry in entrys.iter() {
                    if entry.metadata().unwrap().is_dir() {
                        let path = entry.path();
                        let path = path.to_string_lossy().to_string();
                        under_quae.push(path);
                    }
                }
            }
        };

    // 파일 출력
    while let Some(path) = under_quae.pop() {
        print_dir(&mut bw, Path::new(&path), &mut under_quae);
        writeln!(bw, "").unwrap();
    }
    bw.flush().unwrap();
}

fn print_permition(permition: &str) -> String {
    let mut result = String::new();
    for c in permition.chars() {
        match c {
            '0' => result.push_str("---"),
            '1' => result.push_str("--x"),
            '2' => result.push_str("-w-"),
            '3' => result.push_str("-wx"),
            '4' => result.push_str("r--"),
            '5' => result.push_str("r-x"),
            '6' => result.push_str("rw-"),
            '7' => result.push_str("rwx"),
            _ => {}
        }
    }

    result
}
