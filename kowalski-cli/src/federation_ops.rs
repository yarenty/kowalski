//! `kowalski-cli federation *` operators.

#[cfg(feature = "postgres")]
use kowalski_core::federation::MessageBroker;

/// Send a test [`AclEnvelope`] with [`AclMessage::Ping`] via Postgres `NOTIFY` (channel `kowalski_federation`).
pub async fn run_ping_notify(config_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "postgres")]
    {
        let path = crate::ops::mcp_config_path(config_path);
        let cfg = crate::ops::load_kowalski_config_for_serve(&path)?;
        let url = cfg
            .memory
            .database_url
            .as_ref()
            .ok_or("memory.database_url not set in config")?;
        if !kowalski_core::config::memory_uses_postgres(&cfg.memory) {
            return Err("memory.database_url must be postgres:// or postgresql://".into());
        }
        let pool = kowalski_core::pg_pool_connect(url).await?;
        let pg = kowalski_core::PgBroker::new(pool, "kowalski_federation");
        let env = kowalski_core::AclEnvelope::new(
            "federation",
            "cli-federation-ping",
            kowalski_core::AclMessage::Ping {
                text: "federation ping-notify".into(),
            },
        );
        pg.publish(&env).await?;
        println!("OK — pg_notify on channel kowalski_federation (Ping ACL)");
        Ok(())
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = config_path;
        Err(
            "rebuild with: cargo build -p kowalski-cli --features postgres (and set memory.database_url)"
                .into(),
        )
    }
}
