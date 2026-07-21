use crate::state::appstate::AppState;
use log::{info, warn};
use tauri::{AppHandle, Manager};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteAccess {
    ReadOnly,
    Editable,
}

impl WriteAccess {
    pub fn from_can_write(can_write: bool) -> Self {
        if can_write {
            WriteAccess::Editable
        } else {
            WriteAccess::ReadOnly
        }
    }
}

#[derive(Clone)]
pub enum AppRole {
    Undecided,
    Starting,
    Host {
        room_id: String,
        lan_url: Option<String>,
        public_url: Option<String>,
        local_room_url: String,
        public_room_url: Option<String>,
    },
    Guest {
        room_id: String,
        server_url: String,
        write_access: WriteAccess,
    },
}

#[derive(Debug)]
pub struct TransitionError {
    from: &'static str,
    to: &'static str,
}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid role transition: {} → {}", self.from, self.to)
    }
}

impl From<TransitionError> for String {
    fn from(e: TransitionError) -> String {
        e.to_string()
    }
}

pub struct StartingGuard {
    app: AppHandle,
    completed: bool,
}

impl Drop for StartingGuard {
    fn drop(&mut self) {
        if !self.completed {
            let _ = self
                .app
                .state::<AppState>()
                .transition_role(AppRole::Undecided);
        }
    }
}

impl AppRole {
    pub fn status(&self) -> &'static str {
        match self {
            Self::Undecided => "idle",
            Self::Starting => "starting",
            Self::Host { .. } => "host",
            Self::Guest { .. } => "guest",
        }
    }

    pub fn try_transition(&mut self, new_role: AppRole) -> Result<AppRole, TransitionError> {
        match (&*self, &new_role) {
            (AppRole::Undecided, AppRole::Starting)
            | (AppRole::Starting, AppRole::Host { .. })
            | (AppRole::Starting, AppRole::Guest { .. })
            | (AppRole::Starting, AppRole::Undecided)
            | (AppRole::Host { .. }, AppRole::Undecided)
            | (AppRole::Guest { .. }, AppRole::Undecided) => Ok(std::mem::replace(self, new_role)),
            _ => Err(TransitionError {
                from: self.status(),
                to: new_role.status(),
            }),
        }
    }
}

impl AppState {
    pub fn transition_role(&self, new_role: AppRole) -> Result<AppRole, TransitionError> {
        self.role.lock().unwrap().try_transition(new_role)
    }

    pub fn begin_session(&self, app: AppHandle) -> Result<StartingGuard, String> {
        self.transition_role(AppRole::Starting)
            .map_err(|_| "A session is already active".to_string())?;
        Ok(StartingGuard {
            app,
            completed: false,
        })
    }

    pub fn complete_host(
        &self,
        mut guard: StartingGuard,
        room_id: String,
        lan_url: Option<String>,
        public_url: Option<String>,
        local_room_url: String,
        public_room_url: Option<String>,
    ) -> Result<(), String> {
        match self.transition_role(AppRole::Host {
            room_id,
            lan_url,
            public_url,
            local_room_url,
            public_room_url,
        }) {
            Ok(_) => {
                guard.completed = true;
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn complete_guest(
        &self,
        mut guard: StartingGuard,
        room_id: String,
        server_url: String,
    ) -> Result<(), String> {
        match self.transition_role(AppRole::Guest {
            room_id,
            server_url,
            write_access: WriteAccess::ReadOnly,
        }) {
            Ok(_) => {
                guard.completed = true;
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn current_role(&self) -> AppRole {
        self.role.lock().unwrap().clone()
    }

    pub fn is_host(&self) -> bool {
        matches!(*self.role.lock().unwrap(), AppRole::Host { .. })
    }

    pub fn can_write(&self) -> bool {
        match &*self.role.lock().unwrap() {
            AppRole::Guest { write_access, .. } => *write_access == WriteAccess::Editable,
            _ => true,
        }
    }

    pub fn set_write_access(&self, access: WriteAccess) -> bool {
        let mut role = self.role.lock().unwrap();
        if let AppRole::Guest { write_access, .. } = &mut *role {
            if *write_access != access {
                info!("guest write access changed: {access:?}");
            }
            *write_access = access;
            true
        } else {
            false
        }
    }

    pub fn store_public_url(&self, url: String) {
        {
            let mut role = self.role.lock().unwrap();
            if let AppRole::Host { public_url, .. } = &mut *role {
                *public_url = Some(url.clone());
                info!("host public URL stored from cloudflared");
            } else {
                warn!("cloudflared public URL ignored because role is not Host");
            }
        }
        self.processes.lock().unwrap().tunnel_public_url = Some(url);
    }
}
