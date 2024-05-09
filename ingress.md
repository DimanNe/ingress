#### Setup kind cluster

```fish
ing_create_kind_cluster_and_tag
ing_make_local_registry_accessible_in_kind
```

#### Rollout new version of back/front

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


#### front & back (only once)

```fish
kubectl apply -f ~/devel/ingress/front-permissions.yml
# deploy as described above
kubectl apply -f ~/devel/ingress/front-service.yml
kubectl apply -f ~/devel/ingress/back-service.yml
```

Test if works:

* logs: `kubectl logs -f front-69448bc67d-txlc2 -c front`
* request: `curl -v http://(docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' ingki-worker2):31003`



#### Debug node

```
apt update && apt install -y traceroute dnsutils curl netcat-openbsd iproute2 iputils-ping iptables net-tools telnet tcpdump wget
``
