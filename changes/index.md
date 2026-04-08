Git tag: [{{ release_tag }}](https://github.com/midnightntwrk/midnight-node/tree/{{ release_tag }})

## Docker Images

### DockerHub
{{#if node_docker_image}}
- [midnight-node](https://hub.docker.com/r/midnightntwrk/midnight-node/)
{{/if}}
{{#if toolkit_docker_image}}
- [midnight-node-toolkit](https://hub.docker.com/r/midnightntwrk/midnight-node-toolkit/)
{{/if}}

```shell
{{#if node_docker_image}}
$ docker pull {{ node_docker_image }}
{{/if}}
{{#if toolkit_docker_image}}
$ docker pull {{ toolkit_docker_image }}
{{/if}}
```
