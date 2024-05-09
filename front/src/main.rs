use anyhow::{anyhow, Context, Result};
use futures::StreamExt;
use futures::TryStreamExt;
use kube::runtime::WatchStreamExt;
use kube::ResourceExt;
use pingora::services::Service;

// ===========================================================================================================
// k8s integration:

/// The reconciler that will be called when either object change
async fn reconcile(g: std::sync::Arc<k8s_openapi::api::networking::v1::Ingress>,
                   ctx: std::sync::Arc<front::proxy::Tx>)
                   -> Result<kube::runtime::controller::Action, kube::runtime::watcher::Error> {
   let first_rule = g.spec.as_ref().unwrap().rules.as_ref().unwrap().first().unwrap().clone();
   log::info!("reconcile is called: {}, first_rule: {first_rule:?}", g.name_any());
   ctx.send(first_rule).await.unwrap();
   Ok(kube::runtime::controller::Action::requeue(core::time::Duration::from_secs(45)))
}

fn error_policy(obj: std::sync::Arc<k8s_openapi::api::networking::v1::Ingress>,
                _error: &kube::runtime::watcher::Error,
                ctx: std::sync::Arc<front::proxy::Tx>)
                -> kube::runtime::controller::Action {
   log::info!("error_policy is called: {}", obj.name_any());
   kube::runtime::controller::Action::requeue(core::time::Duration::from_secs(60))
}

async fn watch_kube(tx: front::proxy::Tx) -> Result<()> {
   log::info!("Starting watching k8s updates...");

   let client = kube::Client::try_default().await?;

   let ingress: kube::api::Api<k8s_openapi::api::networking::v1::Ingress> =
      kube::api::Api::default_namespaced(client);
   let context = std::sync::Arc::new(tx);
   kube::runtime::Controller::new(ingress, kube::runtime::watcher::Config::default()).run(reconcile, error_policy, context)
                                                                      .for_each(|res| async move {
                                                                        println!("reconciliation result {res:?}"); })
                                                                      .await;
   Ok(())
}


// ===========================================================================================================
// run HTTP server & k8s integration:

async fn async_main(shutdown_recv: pingora_core::server::ShutdownWatch) {
   let (tx, rx) = tokio::sync::mpsc::channel(16);
   tokio::spawn(async move { watch_kube(tx).await.unwrap() });

   let fds = pingora::server::Fds::new();
   let fds = Some(std::sync::Arc::new(tokio::sync::Mutex::new(fds)));

   let mut proxy = pingora_core::services::listening::Service::new("Pingora HTTP Proxy Service".into(),
                                                                   front::proxy::MyProxy::from(rx));
   proxy.add_tcp("0.0.0.0:8080");

   let mut service = proxy;

   service.start_service(fds, shutdown_recv).await;
   log::info!("service exited.")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
   let mut builder = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
   builder.format_timestamp_micros();
   builder.init();

   let (_tx, shutdown_recv) = tokio::sync::watch::channel(false);
   log::info!("Server starting");
   let service_runtime = pingora_runtime::Runtime::new_steal(4, "Proxy");
   service_runtime.get_handle()
                  .block_on(async move { async_main(shutdown_recv.clone()).await });
   Ok(())
}
