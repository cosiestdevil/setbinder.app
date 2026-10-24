# Cached Cloud Run builds

The Dockerfile separates dependency compilation with cargo-chef. Cloud Build
publishes the `dependencies` target as `IMAGE:build-deps`, pulls it on subsequent
workers, and builds the application on top. The cache image includes Cargo's
compiled dependencies and downloaded sources; the runtime image includes only
the executable and runtime packages. Inline BuildKit metadata supports cache
imports, while publishing the dependency target retains its intermediate layers.

The first build is cold. Rust image/toolchain, Cargo manifests, Cargo.lock,
release-profile, and cargo-chef version changes can invalidate the dependency
cache. Rust application edits, CSS, templates, and JavaScript should leave it
reusable. The Scryfall dataset is still fetched in the application build; it is
not baked into cargo-chef's dummy dependency build.

## Connect the existing trigger

Adding cloudbuild.yaml does not automatically change an existing inline trigger.
In Cloud Build, edit the existing main-branch deployment trigger:

The current deployment targets are already the defaults in cloudbuild.yaml:

- Project: `secure-way-473321-p1`
- Cloud Run service: `setbinder-app`, region `europe-west1`
- Image: `europe-west1-docker.pkg.dev/secure-way-473321-p1/cloud-run-source-deploy/setbinder.app/setbinder-app`
- Existing trigger: `0a053c11-c87b-418d-8ce0-d342c8a0db55`, region `global`

After pushing these files, open the existing trigger in project
`secure-way-473321-p1` and select **Cloud Build configuration file** with location
**Repository**, path `cloudbuild.yaml`. Keep the current build service account,
repository connection, and main-branch filter. The three substitutions are
already populated with the current target; override them in the trigger only
if a target changes. Save the trigger and run it once to seed the cache.

This is a change to the existing deployment trigger, not a second deployment
trigger. The repository changes alone do not activate this setting.

The build service account needs read/write access to the existing Artifact
Registry repository, permission to update the existing Cloud Run service, and
permission to act as its runtime service account. Reuse the permissions from the
existing deployment; the new config does not change public access or runtime
settings. With a custom build service account, Cloud Logging write access is
also required by CLOUD_LOGGING_ONLY.

For a manual build, supply COMMIT_SHA along with the three custom substitutions.
The image is pushed with that immutable tag before deployment. Invalid or empty
settings fail before building. A missing cache is expected on the first run;
compilation or push failures stop the pipeline before deployment.

## Verify reuse

After connecting the trigger, run a cold build, then a second build changing
only application source or a template. Look for the dependency `cargo chef cook`
step being cached and compare the build-and-push durations in Cloud Build.
Changes to dependencies or the release profile should rebuild that layer.

The cache is a separate Artifact Registry image tagged `build-deps`. Keep this
tag in repository cleanup policies. It can use more storage than the runtime
image. If the cache is deleted, the next build regenerates it.

## Local checks

```sh
cargo test --workspace --locked
node --test rocket-app/tests/browse.test.mjs
python3 -m unittest discover -s ci -p 'test_*.py'
```

To exercise the actual container locally, use Docker with network access:

```sh
docker build --target dependencies -t setbinder-build-deps .
docker build --cache-from setbinder-build-deps -t setbinder .
docker run --rm -p 8080:8080 setbinder
```
