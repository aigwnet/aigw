use daemonize::Daemonize;
use std::ffi::CString;
use std::{fs, path::Path};

/// Start a server instance as a daemon.

pub fn daemonize(user: Option<&String>, group: Option<&String>, pid_file: &str) {

    let daemonize = Daemonize::new()
        .umask(0o007) // allow same group to access files but not everyone else
        .pid_file(pid_file);

    let daemonize = match user {
        Some(user) => {
            let user_cstr = CString::new(user.as_str()).unwrap();

            #[cfg(target_os = "macos")]
            let group_id = unsafe { gid_for_username(&user_cstr).map(|gid| gid as i32) };
            #[cfg(target_os = "freebsd")]
            let group_id = unsafe { gid_for_username(&user_cstr).map(|gid| gid as u32) };
            #[cfg(target_os = "linux")]
            let group_id = unsafe { gid_for_username(&user_cstr) };

            daemonize
                .privileged_action(move || {
                    if let Some(gid) = group_id {
                        // Set the supplemental group privileges for the child process.
                        unsafe {
                            libc::initgroups(user_cstr.as_ptr() as *const libc::c_char, gid);
                        }
                    }
                })
                .user(user.as_str())
                .chown_pid_file(true)
        }
        None => daemonize,
    };

    let daemonize = match group {
        Some(group) => daemonize.group(group.as_str()),
        None => daemonize,
    };

    move_old_pid(pid_file);

    daemonize.start().unwrap(); // hard crash when fail
}

fn move_old_pid(path: &str) {
    if !Path::new(path).exists() {
        return;
    }
    let new_path = format!("{path}.old");
    let _ = fs::rename(path, &new_path);
}

unsafe fn gid_for_username(name: &CString) -> Option<libc::gid_t> {
    unsafe {
        let passwd = libc::getpwnam(name.as_ptr() as *const libc::c_char);
        if !passwd.is_null() {
            return Some((*passwd).pw_gid);
        }
    }
    None
}
