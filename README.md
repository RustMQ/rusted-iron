# Rusted Iron

Iron MQ build with Rust

## Prerequisites
We used [Rocket](https://rocket.rs) which is required nighlty version of Rust.
Once `rustup` is installed, configure Rust nightly as your default toolchain by running the command:
```
rustup default nightly
```
If you prefer, once we setup a project directory in the following section, you can use per-directory overrides to use the nightly version only for your Rocket project by running the following command in the directory:
```
rustup override set nightly
```

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
$ SET queue:1:msg:counter 1
$ HMSET queue:1 class "pull" totalrecv 0 totalsent 0
```

**Note**: For integration testing we need to have an up and running Redis DB instance with initialized state.
