# Deployment

## Heroku

Heroku Container Registry allows you to deploy your Docker-based app to Heroku.

Let's assume that you Docker images already prepared under names: `rustmq/web` and `rustmq/pusher`.

### Pushing image(s)

To push an image to Heroku, such as one pulled from Docker Hub, tag it and push it according to this naming template:

```
$ docker tag rustmq/web:v<package_version> registry.heroku.com/<app>/<process-type>     # example: docker tag rustmq/web:v0.0.1 registry.heroku.com/rusted-iron/web
$ docker push registry.heroku.com/<app>/<process-type>                                  # example: docker push registry.heroku.com/rusted-iron/web
```

**Note:** Last command will initiate a deployment on your heroku instance.

More details could be found in [Heroky Container Registry Docs](https://devcenter.heroku.com/articles/container-registry-and-runtime)
