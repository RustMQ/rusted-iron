# Rusted IronMQ

IronMQ build with Rust

## Prerequisites

## Heroku

### Deployment
Application have two separated services: `rusted-iron/web` and `rusted-iron/pusher`.
First of all you have to build both of them:
```
$ docker build -f <docker_file> -t rusted-iron/web:v<package_version> .                 # example: docker build -f Dockerfile.web -t rusted-iron/web:v0.0.1 .
$ docker build -f <docker_file> -t rusted-iron/pusher:v<package_version> .              # example: docker build -f Dockerfile.pusher -t rusted-iron/pusher:v0.0.1 .
```
To push an image to Heroku, such as one pulled from Docker Hub, tag it and push it according to this naming template:
```
$ docker tag rusted-iron:v<package_version> registry.heroku.com/<app>/<process-type>    # example: docker tag rusted-iron:v0.0.1 registry.heroku.com/rusted-iron/web
$ docker push registry.heroku.com/<app>/<process-type>                                  # example: docker push registry.heroku.com/rusted-iron/web
```

**Note:** Last command will initiate a deployment on your heroku instance.


### Local testing
#### Running containers
`Dockerfile` contains environment variables.
When running a Docker container locally, you can set an environment variable using the `-e` flag or `--env-file`.
List of environment variables:
```
PORT=<port>
REDISCLOUD_URL=<redis_url>
REDIS_CONNECTION_MAX_SIZE=<connection_max_size>
RUST_BACKTRACE=<backtrace>
RUST_LOG=<log_name>
SERVICE_TOKEN=<service_token>
```
**Note:** `SERVICE_TOKEN` variable using for `rusted-iron/pusher` only.

After this preparation you can run containers:
```
$ docker-compose up -d
```

### Redis in development
Currently we don't have API or DB migration for Redis, that's why we need manually run commands below to initialize DB state:
```
$ SADD email:<email> <user_id>
$ HMSET user:<some_id> email <email> first_name <user_fn> last_name <user_ln> created_at 2018-04-04T07:10:00.000Z password <bcrypted_password>
```
For password generation any bcrypt online tool could be used, i.e. https://www.dailycred.com/article/bcrypt-calculator

**Note**: For integration testing we need to have an up and running Redis DB instance with initialized state.
