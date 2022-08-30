use std::sync::Arc;

use anyhow::Context;
use clap::Parser;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use conduit_api::router::ConduitApplicationController;
use conduit_core::config::AppConfig;
use conduit_infrastructure::connection_pool::ConduitConnectionManager;
use conduit_infrastructure::service_register::ServiceRegister;
use conduit_infrastructure::services::utils::conduit_seed_service::ConduitSeedService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    // main入口
    // 加载.evn中的配置
    dotenv::dotenv().ok();

    // 读取配置到config
    let config = Arc::new(AppConfig::parse());

    // 设置日志的格式
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(&config.rust_log))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("environment loaded and configuration parsed, initializing Postgres connection and running migrations...");
    // 创建数据库链接池
    let pg_pool = ConduitConnectionManager::new_pool(&config.database_url, config.run_migrations)
        .await
        .expect("could not initialize the database connection pool");

    let port = config.port;

    // 注册所有的服务
    let service_register = ServiceRegister::new(pg_pool, config.clone());


    // 初始化，如果有测试的数据了就不再初始化
    if config.seed {
        info!("seeding enabled, creating test data...");
        ConduitSeedService::new(service_register.clone())
            .seed()
            .await
            .expect("unexpected error occurred while seeding application data");
    }

    info!("migrations successfully ran, initializing axum server...");
    // 启动服务器
    ConduitApplicationController::serve(port, &config.cors_origin, service_register)
        .await
        .context("could not initialize application routes")?;

    Ok(())
}
