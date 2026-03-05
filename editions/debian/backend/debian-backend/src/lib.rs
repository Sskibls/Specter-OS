use anyhow::Result;
use gk_platform_api::{PlatformBackend, UpdateBundle};

pub struct DebianBackend;

impl PlatformBackend for DebianBackend {
    fn refresh_metadata(&self) -> Result<()> {
        Ok(())
    }

    fn download_bundle(&self, channel: &str) -> Result<UpdateBundle> {
        Ok(UpdateBundle {
            id: "debian-bundle-stub".to_string(),
            channel: channel.to_string(),
        })
    }

    fn verify_bundle(&self, _bundle: &UpdateBundle) -> Result<()> {
        Ok(())
    }

    fn stage(&self, _bundle: &UpdateBundle) -> Result<()> {
        Ok(())
    }

    fn commit(&self) -> Result<()> {
        Ok(())
    }

    fn rollback(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debian_backend_stub_flow() {
        let backend = DebianBackend;
        backend.refresh_metadata().expect("refresh should succeed");
        let bundle = backend
            .download_bundle("stable")
            .expect("download should succeed");
        backend
            .verify_bundle(&bundle)
            .expect("verify should succeed");
    }
}
