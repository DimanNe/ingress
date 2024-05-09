pub struct MyBackendService {}

#[tonic::async_trait]
impl grpc::back::backend_server::Backend for MyBackendService {
   async fn say_hello(&self,
                      request: tonic::Request<grpc::back::HelloReq>)
                      -> Result<tonic::Response<grpc::back::HelloResp>, tonic::Status> {
      let hostname = hostname::get().unwrap();
      let now = chrono::Local::now().format("%H:%M:%S");
      let resp = format!("Hello from server: {hostname:?} at: {now} for req: {}", request.into_inner().req).into();
      let resp = grpc::back::HelloResp { resp };
      Ok(tonic::Response::new(resp))
   }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
   let addr = "0.0.0.0:50055".parse().unwrap();
   let server = grpc::back::backend_server::BackendServer::new(MyBackendService {});
   tonic::transport::Server::builder().add_service(server).serve(addr).await?;
   Ok(())
}
