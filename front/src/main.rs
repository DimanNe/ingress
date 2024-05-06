#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
   let mut client = grpc::back::backend_client::BackendClient::connect("http://127.0.0.1:50055").await?;
   let request = tonic::Request::new(grpc::back::HelloReq { req: "asdf".into() });
   let response = client.say_hello(request).await?;
   println!("RESPONSE={:?}", response.into_inner().resp);
   Ok(())
}
