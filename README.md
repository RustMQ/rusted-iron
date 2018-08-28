# Rusted Iron

Iron MQ build with Rust

## Prerequisites

Current application used **stable** [Rust](https://www.rust-lang.org/en-US/). At this moment we use latest stable version of Rust **1.28**.
Also, as DB we used [Redis](https://redis.io). At this moment we use latest stable version of Rust **4.0.9**.

## Project layout

This project contains:

* [`queue`]: Common structures and traits

* [`pusher`]: Push delivery service

* [`web`]: API and backend for pull/push queues

## Local development

### Running application

First of all, we asume that you have Redis instance up and running.
Both applications requires environment variables to be set, list of variables could be found:

* [`env.pusher.list`]: pusher environment variables

* [`env.web.list`]: web environment variables

To run application you should use `cargo` command:

Use `cargo run -p <service_name>`, where service name is `web` or `pusher`.

### Running using Docker

We prepared `docker-compose-development.yml` file which describes current application layout. To run it, just type in command line:
>$ `docker-compose -f ./docker-compose-development.yml up -d`

Make sure that you are using latest images with all you code changes. For this you could run `docker-compose -f ./docker-compose-development.yml build` command

## Authentication (Disabled)

**Authentication is disabled for now**, but in case if it will be enabled then you should do some manual work.
Currently we don't have scripts that could initialize Redis, that's why we need manually run commands below to initialize DB state:

>$ SADD email:<email> <user_id>

>$ HMSET user:<some_id> email <email> first_name <user_fn> last_name <user_ln> created_at 2018-04-04T07:10:00.000Z password <bcrypted_password>

For password generation any [bcrypt online tool](https://www.dailycred.com/article/bcrypt-calculator)

## Heroku deployments

Currently we are using Docker-based deployments. More details could be found on link [Deploying with Docker](https://devcenter.heroku.com/categories/deploying-with-docker).
