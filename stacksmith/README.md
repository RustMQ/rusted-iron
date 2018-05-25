# Bitnami Stacksmith support

This document describe the general process on how to package application using Bitnami Stacksmith

## Package and deploy with Stacksmith

1. Go to [stacksmith.bitnami.com](https://stacksmith.bitnami.com).
2. Create a new application and select the `Generic application (no pre-installed runtime)` stack template.
3. Select the targets you are interested on (AWS, Kubernetes,...).
4. Compress this repo and upload it as application files:

```bash
   git clone https://github.com/XenonIO/rusted-iron
   tar czf rustmq.tar.gz rusted-iron
   ```

Note: As we have two services, you need to create to application. One for web, and one for pusher. Same archive could be used.

5. Upload respective scripts for web - [_build.sh_](web/build.sh), [_run.sh_](web/run.sh), and for pusher - [_build.sh_](pusher/build.sh), [_run.sh_](pusher/run.sh). _run.sh_ file contain sensitive information which should be updated before upload. For example, redis connection url.
6. Click the <kbd>Create</kbd> button.
7. Wait for app to be built and deploy it in your favorite target platform.

## Scripts

In the stacksmith folder, you can find the required scripts to build and run this application:

### build.sh

This script takes care of installing the application and its dependencies. It performs the next steps:

* Adds the system user that will run the application.
* Uncompress the application code to the `/opt/rusted-iron` folder.
* Adjust the application files permissions.
* Install dependencies.

### boot.sh

Not done yet. We need to rethink the process on how to deliver sensitive information like db connection properties, ports, logs, and other config

### run.sh

This script just starts the application with the proper user.
