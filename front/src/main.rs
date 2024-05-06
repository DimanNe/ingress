use pingora::services::Service;

pub fn init_logger(log_level: &str) {
   let mut builder = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level));
   builder.format_timestamp_micros();
   builder.init();
}


struct MyProxy {
   shutdown: tokio::sync::Notify,
   client:   std::sync::Arc<tokio::sync::Mutex<Option<grpc::back::backend_client::BackendClient<tonic::transport::Channel>>>>,
}

impl MyProxy {
   fn new() -> std::sync::Arc<Self> {
      std::sync::Arc::new(MyProxy { shutdown: tokio::sync::Notify::new(),
                                    client:   std::sync::Arc::new(tokio::sync::Mutex::new(None)), })
   }
   async fn init_client(&self) {
      let mut guard = self.client.lock().await;
      if guard.is_some() {
         return;
      }
      let client = grpc::back::backend_client::BackendClient::connect("http://127.0.0.1:50055").await
                                                                                               .unwrap();
      *guard = Some(client);
   }
   async fn try_reading_headers(&self,
                                mut downstream_session: Box<pingora_core::protocols::http::server::Session>)
                                -> Option<Box<pingora_core::protocols::http::server::Session>> {
      // phase 1 read request header
      match downstream_session.read_request().await {
         Ok(true) => {
            log::debug!("Successfully get a new request");
         }
         Ok(false) => {
            return None; // TODO: close connection?
         }
         Err(mut e) => {
            e.as_down();
            log::error!("Fail to proxy: {}", e);
            if matches!(e.etype, pingora_core::ErrorType::InvalidHTTPHeader) {
               downstream_session.respond_error(400).await;
            } // otherwise the connection must be broken, no need to send anything
            downstream_session.shutdown().await;
            return None;
         }
      }
      log::info!("Request header: {:?}", downstream_session.req_header().as_ref());
      Some(downstream_session)
   }

   async fn process_request(self: &std::sync::Arc<Self>,
                            mut session: pingora_core::protocols::http::server::Session)
                            -> Option<pingora_core::protocols::Stream> {
      self.init_client().await;

      let mut client_guard = self.client.lock().await;
      let client = client_guard.as_mut().unwrap();
      let request = tonic::Request::new(grpc::back::HelloReq { req: "asdf".into() });
      let response = client.say_hello(request).await.unwrap();
      let response = response.into_inner().resp;
      log::info!("RESPONSE={:?}", response);

      let mut headers = pingora_http::ResponseHeader::build(http::status::StatusCode::OK, Some(0)).unwrap();
      headers.insert_header(http::header::CONTENT_LENGTH, response.len().to_string()).unwrap();
      headers.insert_header(http::header::CONTENT_TYPE, "text/plain").unwrap();
      let _ = session.write_response_header(Box::new(headers)).await;
      let _ = session.write_response_body(response.into()).await;
      return session.finish().await.ok().flatten();
   }
}


#[async_trait::async_trait]
impl pingora_core::apps::HttpServerApp for MyProxy {
   async fn process_new_http(self: &std::sync::Arc<Self>,
                             session: pingora_core::protocols::http::server::Session,
                             shutdown: &pingora_core::server::ShutdownWatch)
                             -> Option<pingora_core::protocols::Stream> {
      let session = Box::new(session);

      // TODO: keepalive pool, use stack
      let mut session = match self.try_reading_headers(session).await {
         Some(downstream_session) => downstream_session,
         None => return None, // bad request
      };

      if *shutdown.borrow() {
         // stop downstream from reusing if this service is shutting down soon
         session.set_keepalive(None);
      } else {
         // default 60s
         session.set_keepalive(Some(60));
      }

      self.process_request(*session).await
   }

   fn http_cleanup(&self) { self.shutdown.notify_waiters(); }
}



async fn async_main(shutdown_recv: pingora_core::server::ShutdownWatch) {
   let fds = pingora::server::Fds::new();
   let fds = Some(std::sync::Arc::new(tokio::sync::Mutex::new(fds)));

   let mut proxy =
      pingora_core::services::listening::Service::new("Pingora HTTP Proxy Service".into(), MyProxy::new());
   proxy.add_tcp("0.0.0.0:8080");

   let mut service = proxy;

   service.start_service(fds, shutdown_recv).await;
   log::info!("service exited.")
}

// #[tokio::main(flavor = "multi_thread", worker_threads = 4)]
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
