
## **Preparation**

#### **Setup kind cluster**


#### **New version of back/front pod**

```fish
set app front; set v (date -u +%Y.%m.%d--%H.%M.%S)
docker build -t localhost:5001/$app:$v -f ~/devel/ingress/$app.Dockerfile ~/devel/ingress/
docker push localhost:5001/$app:$v
yq e ".spec.template.spec.containers[0].image = \"localhost:5001/$app:$v\"" -i ~/devel/ingress/$app-deployment.yml
kubectl apply -f ~/devel/ingress/$app-deployment.yml
sleep 1
set pod (kubectl get pods --selector=app=$app --sort-by=.metadata.creationTimestamp -o jsonpath='{.items[-1:].metadata.name}')
kubectl logs -f $pod -c $app
```

#### **Other k8s resources**

```fish
kubectl apply -f ~/devel/ingress/front-permissions.yml
kubectl apply -f ~/devel/ingress/front-service.yml
kubectl apply -f ~/devel/ingress/back-service.yml
```


--------------------------------------------------------------------------------------------------------------
## **Demo**

`back` is a grpc services, listens on 50055 port, replies with a string: `format!("Hello from server: {hostname:?} at: {now} for req: {}", request.into_inner().req).into();`. See back/src/main.rs.

`front` service is minimal possible implementation of ingress-controller. See front/src/main.rs & front/src/proxy.rs.

The service:

* listens on 80 port, using cloudflare's Pingora rust server
* watches for changes in k8s configuration (`fn watch_kube(tx: front::proxy::Tx)`)
* matches host & path from http request with what is specified in ingress.yml
  (`if http_host == host && http_path == path`), and if everything matches, makes
  grpc request to the backend from ingress.yml receives response, and creates http response



#### No ingress config loaded:

```
curl -H "Host: asdf.com" http://172.18.0.4:31003/qwer
No rules has been set yet! Apply ingress.yml first!
```

#### Load config, but host & path do not match

```
kubectl apply -f ~/devel/ingress/test-ingress.yml
ingress.networking.k8s.io/minimal-ingress created
```
where config is:

```yml
  - host: "foo.bar.com"
    http:
      paths:
      - pathType: Prefix
        path: "/bar"
        backend:
          service:
            name: back
            port:
              number: 50055
```

Since path & host did not match request was not proxied to backend service:

```
curl -H "Host: asdf.com" http://172.18.0.4:31003/qwer
Ingress rule: host: foo.bar.com, path: /bar => mapped to => back:50055
Current HTTP request path /qwer, host: asdf.com
```

#### Make curl request to "correct" path & host (from ingress config):


First curl request was processed by back-b9fc48c58-8rn2h:
```
curl -H "Host: foo.bar.com" http://172.18.0.4:31003/bar
Ingress rule: host: foo.bar.com, path: /bar => mapped to => back:50055
Current HTTP request path /bar, host: foo.bar.com
Host & path match => forwarding reqeust to backend: http://back:50055...
Hello from server: "back-b9fc48c58-8rn2h" at: 16:36:16 for req: asdf
```

Second curl request was processed by back-b9fc48c58-zwfwx:
```
curl -H "Host: foo.bar.com" http://172.18.0.4:31003/bar
Ingress rule: host: foo.bar.com, path: /bar => mapped to => back:50055
Current HTTP request path /bar, host: foo.bar.com
Host & path match => forwarding reqeust to backend: http://back:50055...
Hello from server: "back-b9fc48c58-zwfwx" at: 16:37:20 for req: asdf
```
