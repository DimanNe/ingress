use k8s_openapi::api::networking::v1::IngressRule;
pub type Tx = tokio::sync::mpsc::Sender<IngressRule>;
pub type Rx = tokio::sync::mpsc::Receiver<IngressRule>;

struct MutState {
   rx:   Rx,
   rule: Option<IngressRule>,
}

pub struct MyProxy {
   shutdown: tokio::sync::Notify,
   state:    tokio::sync::Mutex<std::cell::RefCell<MutState>>,
}

impl MyProxy {
   pub fn from(rx: Rx) -> std::sync::Arc<Self> {
      std::sync::Arc::new(MyProxy { shutdown: tokio::sync::Notify::new(),
                                    state:
                                       tokio::sync::Mutex::new(std::cell::RefCell::new(MutState { rx,
                                                                                                  rule:
                                                                                                     None })), })
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

   async fn dispatch(self: &std::sync::Arc<Self>,
                     session: &mut pingora_core::protocols::http::server::Session)
                     -> String {
      // String::new()
      let rule = {
         let state = self.state.lock().await;
         let mut state = state.borrow_mut();

         if let Ok(new_rule) = state.rx.try_recv() {
            state.rule = Some(new_rule);
         }
         state.rule.clone()
      };
      if rule.is_none() {
         return "No rules has been set yet! Apply ingress.yml first!".to_string();
      }
      let rule = rule.unwrap();
      let host = rule.host.unwrap();
      let first_path = rule.http.as_ref().unwrap().paths.first().unwrap();
      let path = first_path.path.as_ref().unwrap();
      let service_name = &first_path.backend.service.as_ref().unwrap().name;
      let service_port = first_path.backend.service.as_ref().unwrap().port.as_ref().unwrap().number.unwrap();

      let http_path = session.req_header().uri.path();
      let http_host = session.req_header().headers.get(http::header::HOST).unwrap().to_str().unwrap();
      let mut resp = format!(
                             "
Ingress rule: host: {host}, path: {path} => mapped to => {service_name}:{service_port}
Current HTTP request path {http_path}, host: {http_host}\n"
      );

      if http_host == host && http_path == path {
         let back = format!("http://{service_name}:{service_port}");
         resp.push_str(&format!("Host & path match => forwarding reqeust to backend: {back}...\n"));
         let endpoint = tonic::transport::channel::Endpoint::from_shared(back).unwrap();
         let mut client = grpc::back::backend_client::BackendClient::connect(endpoint).await.unwrap();
         let request = tonic::Request::new(grpc::back::HelloReq { req: "asdf".into() });
         let response = client.say_hello(request).await.unwrap();
         let response = response.into_inner().resp;
         resp.push_str(&response);
         // log::info!("RESPONSE={:?}", response);
      }
      resp
   }
}


#[async_trait::async_trait]
impl pingora_core::apps::HttpServerApp for MyProxy {
   async fn process_new_http(self: &std::sync::Arc<Self>,
                             session: pingora_core::protocols::http::server::Session,
                             shutdown: &pingora_core::server::ShutdownWatch)
                             -> Option<pingora_core::protocols::Stream> {
      let session = Box::new(session);

      let mut session = match self.try_reading_headers(session).await {
         Some(downstream_session) => downstream_session,
         None => return None, // bad request
      };
      session.set_keepalive(if *shutdown.borrow() { None } else { Some(60) });

      // self.process_request(*session).await
      let response = self.dispatch(&mut session).await;
      let mut headers = pingora_http::ResponseHeader::build(http::status::StatusCode::OK, Some(0)).unwrap();
      headers.insert_header(http::header::CONTENT_LENGTH, response.len().to_string()).unwrap();
      headers.insert_header(http::header::CONTENT_TYPE, "text/plain").unwrap();
      let _ = session.write_response_header(Box::new(headers)).await;
      let _ = session.write_response_body(response.into()).await;
      return session.finish().await.ok().flatten();
   }

   fn http_cleanup(&self) { self.shutdown.notify_waiters(); }
}
