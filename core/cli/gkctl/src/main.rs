use std::path::Path;
use std::process::ExitCode;

use gk_audit::AuditStore;

fn main() -> ExitCode {
    let mut args = std::env::args();
    let _binary = args.next();
    let command = args.next().unwrap_or_else(|| "status".to_string());

    match command.as_str() {
        "status" => {
            println!("gkctl status: stub");
            ExitCode::SUCCESS
        }
        "panic" => {
            println!("gkctl panic: stub");
            ExitCode::SUCCESS
        }
        "audit-verify" => {
            let path = args
                .next()
                .unwrap_or_else(|| "/var/lib/phantomkernel/auditd/chain.log".to_string());
            verify_audit_chain(Path::new(&path))
        }
        other => {
            println!("gkctl unknown command: {other}");
            ExitCode::from(1)
        }
    }
}

fn verify_audit_chain(path: &Path) -> ExitCode {
    let store = match AuditStore::open(path) {
        Ok(store) => store,
        Err(error) => {
            eprintln!(
                "failed to open audit chain store {}: {error}",
                path.display()
            );
            return ExitCode::from(1);
        }
    };

    match store.replay_and_verify() {
        Ok(events) => {
            println!(
                "gkctl audit-verify: chain valid ({} events) [{}]",
                events.len(),
                path.display()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!(
                "gkctl audit-verify: chain invalid [{}]: {error}",
                path.display()
            );
            ExitCode::from(1)
        }
    }
}
