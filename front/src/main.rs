use pingora::services::Service;

pub fn init_logger(log_level: &str) {
   let mut builder = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level));
   builder.format_timestamp_micros();
   builder.init();
}


async fn watch_kube() {
    log::info!("Starting watching k8s updates...");
}

async fn async_main(shutdown_recv: pingora_core::server::ShutdownWatch) {
   tokio::spawn(async move { watch_kube().await });

   let fds = pingora::server::Fds::new();
   let fds = Some(std::sync::Arc::new(tokio::sync::Mutex::new(fds)));

   let mut proxy = pingora_core::services::listening::Service::new("Pingora HTTP Proxy Service".into(),
                                                                   front::proxy::MyProxy::new());
   proxy.add_tcp("0.0.0.0:8080");

   let mut service = proxy;

   service.start_service(fds, shutdown_recv).await;
   log::info!("service exited.")
}



fn main() -> Result<(), Box<dyn std::error::Error>> {
   init_logger(&"info");
   //    let opt = pingora_core::server::configuration::Opt { upgrade:   false,
   //                                                         daemon:    false,
   //                                                         nocapture: false,
   //                                                         test:      false,
   //                                                         conf:      None, };

   // let mut server = pingora_core::server::Server::new(Some(opt)).unwrap();
   let (_tx, shutdown_recv) = tokio::sync::watch::channel(false);
   // let configuration = opt.conf.as_ref().map_or_else(|| { pingora_core::server::configuration::ServerConf::new_with_opt_override(&opt).unwrap() },
   //                                                           |_| { pingora_core::server::configuration::ServerConf::load_yaml_with_opt_override(&opt).unwrap() });
   // let configuration = std::sync::Arc::new(configuration);
   // let options = opt;
   // server.bootstrap();
   // let mut services: Vec<Box<dyn pingora_core::services::Service>> = Default::default();
   // server.add_service(proxy);
   // services.push(Box::new(proxy));
   // server.run_forever();
   // let conf = configuration.as_ref();
   log::info!("Server starting");
   let service_runtime = pingora_runtime::Runtime::new_steal(4, "Proxy");
   service_runtime.get_handle()
                  .block_on(async move { async_main(shutdown_recv.clone()).await });
   Ok(())
}
