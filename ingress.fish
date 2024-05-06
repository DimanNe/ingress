set cluster_name                 ingki
set reg_name                     ingki
set reg_port                     5001



# ============================================================================================================
# KIND setup

function ing_start_kind_docker_registry
   set -l exists (docker inspect -f '{{.Id}}' $reg_name 2>/dev/null)
   if test -z "$exists" # Does not exist
      docker run -it -p "127.0.0.1:$reg_port:5000" --network bridge --name $reg_name registry:2
   else
      docker start -ai $reg_name
   end
end


function ing_delete_kind_cluster
   kind delete cluster --name $cluster_name --verbosity 2
end

function ing_create_kind_cluster_and_tag

   # Explanation of the configuration:
   # The Configuration Flow
   # * Request to Fetch an Image: When Kubernetes (via containerd) needs to pull an image, it interprets the image name,
   #   which includes the registry from where the image should be fetched. If the registry is not specified in the image
   #   name, it defaults to Docker Hub. However, in your setup and many local development environments, the image name
   #   includes a local registry (like localhost:5001/my-image).
   # * containerd Looks for Configuration: Based on the registry's hostname and port (e.g., localhost:5001), containerd
   #   will look for a configuration directory matching this registry endpoint. This is why the directory structure you
   #   mentioned, /etc/containerd/certs.d/localhost:5001, is important—it's where containerd expects to find configuration
   #   files for the localhost:5001 registry.
   # * Reading hosts.toml: Within the directory /etc/containerd/certs.d/localhost:5001, containerd looks for the hosts.toml
   #   file. This file contains settings specific to this registry endpoint. The file structure and content dictate how
   #   containerd should handle requests to this endpoint.
   #
   # Example of hosts.toml Contents
   # * Given an entry like [host."http://kind-registry:5000"] in hosts.toml, here’s how containerd interprets it:
   # * Host Directive: The host key specifies a hostname with a protocol and potentially a port. This is the actual address
   #   that containerd will use to pull images.
   # * Mapping: The mapping tells containerd that whenever it gets a request to pull an image from localhost:5001, it
   #   should instead redirect this request to http://kind-registry:5000.


   # https://kind.sigs.k8s.io/docs/user/local-registry/
   echo -e "\n"(set_color -o bryellow)"Creating kind cluster: $cluster_name..."(set_color normal)
   echo "
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
name: $cluster_name

networking:
   # WARNING: It is _strongly_ recommended that you keep this the default
   # (127.0.0.1) for security reasons. However it is possible to change this.
   # Uncomment the line that reads (0.0.0.0) to listen on external addresses
   # and gain kubectl remote capability for this cluster (eg. lens), and
   # comment the 127.0.0.1 line.
   apiServerAddress: 127.0.0.1

containerdConfigPatches:
- |-
  [plugins.\"io.containerd.grpc.v1.cri\".registry]
    config_path = \"/etc/containerd/certs.d\"


nodes:
   - role: control-plane
   - role: worker
   - role: worker
   - role: worker
" | kind create cluster --config=- --verbosity 2


   set -l REGISTRY_DIR "/etc/containerd/certs.d/localhost:$reg_port"
   for node in (kind get nodes --name $cluster_name)
     docker exec "$node" mkdir -p "$REGISTRY_DIR"
     echo "
[host.\"http://$reg_name:5000\"]
   " | docker exec -i $node cp /dev/stdin "$REGISTRY_DIR/hosts.toml"
   end

   # Using The Registry
   # The registry can be used like this.
   # First we'll pull an image docker pull gcr.io/google-samples/hello-app:1.0
   # Then we'll tag the image to use the local registry
   # docker tag gcr.io/google-samples/hello-app:1.0 localhost:5001/hello-app:1.0
   # Then we'll push it to the registry docker push localhost:5001/hello-app:1.0
   # And now we can use the image kubectl create deployment hello-server --image=localhost:5001/hello-app:1.0
   #
   # If you build your own image and tag it like localhost:5001/image:foo and then use it in kubernetes as
   # localhost:5001/image:foo. And use it from inside of your cluster application as kind-registry:5000.

   kubectl cluster-info
end


function ing_make_local_registry_accessible_in_kind
   echo -e "\n"(set_color -o bryellow)"Making local docker registry accessible in kind cluster..."(set_color normal)
   set -l networks (docker inspect -f='{{json .NetworkSettings.Networks.kind}}' $reg_name)
   if test "$networks" = "null"
      docker network connect kind $reg_name
   end

   # https://github.com/kubernetes/enhancements/tree/master/keps/sig-cluster-lifecycle/generic/1755-communicating-a-local-registry
   echo "
apiVersion: v1
kind: ConfigMap
metadata:
  name: local-registry-hosting
  namespace: kube-public
data:
  localRegistryHosting.v1: |
    host: localhost:$reg_port
    help: https://kind.sigs.k8s.io/docs/user/local-registry/
" | kubectl apply -f -
end






# ============================================================================================================
#
