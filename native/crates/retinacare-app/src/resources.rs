use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct ResourcePaths {
    pub python: PathBuf,
    pub worker_source: PathBuf,
    pub models: PathBuf,
    pub data: PathBuf,
}

impl ResourcePaths {
    pub fn discover() -> io::Result<Self> {
        let executable = env::current_exe()?;
        let root = executable
            .parent()
            .ok_or_else(|| io::Error::other("installation directory unavailable"))?;
        let data = if cfg!(windows) {
            env::var_os("LOCALAPPDATA").map(PathBuf::from)
        } else {
            env::var_os("XDG_DATA_HOME").map(PathBuf::from).or_else(|| {
                env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
        }
        .ok_or_else(|| io::Error::other("user application-data directory unavailable"))?
        .join("RetinaCare");
        #[cfg(debug_assertions)]
        if let Some(dev_root) = env::var_os("RETINACARE_DEV_ROOT") {
            let root = PathBuf::from(dev_root);
            return Ok(Self {
                python: env::var_os("RETINACARE_PYTHON")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| {
                        PathBuf::from(if cfg!(windows) { "python" } else { "python3" })
                    }),
                worker_source: root.join("ai_engine/src"),
                models: root.join("models"),
                data,
            });
        }
        Ok(Self::installed(root, data))
    }

    pub fn installed(root: &Path, data: PathBuf) -> Self {
        let models = data.join("models");
        Self {
            python: root.join("runtime").join(if cfg!(windows) {
                "python.exe"
            } else {
                "bin/python3"
            }),
            worker_source: root.join("resources/ai_engine/src"),
            models,
            data,
        }
    }

    pub fn prepare_data(&self) -> io::Result<()> {
        for directory in ["database", "logs", "reports", "temp", "models"] {
            fs::create_dir_all(self.data.join(directory))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installed_paths_handle_spaces_and_unicode() {
        let root = Path::new("portable RetinaCare/आँख");
        let data = PathBuf::from("user data/患者");
        let paths = ResourcePaths::installed(root, data.clone());
        assert!(paths.worker_source.starts_with(root));
        assert_eq!(paths.data, data);
        assert_eq!(paths.models, data.join("models"));
    }
}
