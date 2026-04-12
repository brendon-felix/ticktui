mod app;
mod db;
mod debug;
mod taskparser;
mod tasks;
mod term;
mod ui;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Determine database path: $HOME/.local/share/ticktui/tasks.db
    let db_path = {
        let mut p = dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        p.push("ticktui");
        std::fs::create_dir_all(&p)?;
        p.push("tasks.db");
        p.to_string_lossy().to_string()
    };

    let db = db::Db::open(&db_path)?;

    let mut app = app::App::new(db)?;
    app.run().await?;
    Ok(())
}
