# Rusted Iron

Iron MQ build with Rust

## Prerequisites

## Heroku

### Deployment
To push an image to Heroku, such as one pulled from Docker Hub, tag it and push it according to this naming template:
```
$ docker build -t rusted-iron:v<package_version> .                                      # example: docker build -t rusted-iron:v0.0.1 .
$ docker tag rusted-iron:v<package_version> registry.heroku.com/<app>/<process-type>    # example: docker tag rusted-iron:v0.0.1 registry.heroku.com/rusted-iron/web
$ docker push registry.heroku.com/<app>/<process-type>                                  # example: docker push registry.heroku.com/rusted-iron/web
```

**Note:** Last command will initiate a deployment on your heroku instance.


### Local testing
`Dockerfile` contains environment variables.
When running a Docker container locally, you can set an environment variable using the `-e` flag or `--env-file`:
```
$ docker run -it --rm --link rust-redis:rust-redis --env-file ./env.list --name rusted-iron -p 8000:8000 rusted-iron:latest
```

### Redis in development
Currently we don't have API or DB migration for Redis, that's why we need manually run commands below to initialize DB state:
```
$ SADD email:<email> <user_id>
$ HMSET user:<some_id> email <email> first_name <user_fn> last_name <user_ln> created_at 2018-04-04T07:10:00.000Z password <bcrypted_password>
```
For password generation any bcrypt online tool could be used, i.e. https://www.dailycred.com/article/bcrypt-calculator

**Note**: For integration testing we need to have an up and running Redis DB instance with initialized state.
