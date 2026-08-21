use std::process::{Child, Command};

#[cfg(unix)]
pub(super) fn configure_process_tree(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(windows)]
pub(super) fn configure_process_tree(_command: &mut Command) {}

#[cfg(unix)]
pub(super) struct ProcessTree;

#[cfg(unix)]
impl ProcessTree {
    pub(super) fn attach(_child: &Child) -> Result<Self, String> {
        Ok(Self)
    }

    pub(super) fn terminate(&self, child: &mut Child) {
        let pid = child.id();
        // The child is the leader of a dedicated process group.
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
}

#[cfg(windows)]
pub(super) struct ProcessTree(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl ProcessTree {
    pub(super) fn attach(child: &Child) -> Result<Self, String> {
        use std::mem::{size_of, zeroed};
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::*;
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return Err("failed to create Windows verification job object".to_string());
            }
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ) == 0
                || AssignProcessToJobObject(job, child.as_raw_handle() as _) == 0
            {
                windows_sys::Win32::Foundation::CloseHandle(job);
                return Err(
                    "failed to attach verification process to Windows job object".to_string(),
                );
            }
            Ok(Self(job))
        }
    }

    pub(super) fn terminate(&self, child: &mut Child) {
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.0, 1);
        }
        let _ = child.kill();
    }
}

#[cfg(windows)]
impl Drop for ProcessTree {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
