use anyhow::Result;
use gk_platform_api::{PlatformBackend, UpdateBundle};

pub struct FedoraBackend;

impl PlatformBackend for FedoraBackend {
    fn refresh_metadata(&self) -> Result<()> {
        Ok(())
    }

    fn download_bundle(&self, channel: &str) -> Result<UpdateBundle> {
        Ok(UpdateBundle {
            id: "fedora-bundle-stub".to_string(),
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
    fn fedora_backend_stub_flow() {
        let backend = FedoraBackend;
        backend.refresh_metadata().expect("refresh should succeed");
        let bundle = backend
            .download_bundle("stable")
            .expect("download should succeed");
        backend
            .verify_bundle(&bundle)
            .expect("verify should succeed");
    }
}
