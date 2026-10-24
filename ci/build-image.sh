#!/usr/bin/env bash
set -euo pipefail

# IMAGE is an untagged Artifact Registry image path; TAG identifies this build.
: "${IMAGE:?Set IMAGE to the existing Artifact Registry image path (without a tag)}"
: "${TAG:?Set TAG to a commit SHA or another unique image tag}"
if [[ "$IMAGE" != *-docker.pkg.dev/*/*/* || "$IMAGE" == *:* || "$IMAGE" == *@* ]]; then
  echo 'IMAGE must be an untagged Artifact Registry path: REGION-docker.pkg.dev/PROJECT/REPOSITORY/IMAGE' >&2
  exit 1
fi
if [[ ! "$TAG" =~ ^[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}$ ]]; then
  echo 'TAG is not a valid Docker tag' >&2
  exit 1
fi

cache_image="${IMAGE}:build-deps"
# A new repository has no cache yet. Build failures and push failures below
# remain fatal; only the optional initial cache pull may fail.
if ! docker pull "$cache_image"; then
  echo 'Dependency cache unavailable; building dependencies from scratch.' >&2
fi

docker build \
  --target dependencies \
  --cache-from "$cache_image" \
  --build-arg BUILDKIT_INLINE_CACHE=1 \
  --tag "$cache_image" \
  .
docker push "$cache_image"

# The preceding build also populates the worker's local dependency cache.
docker build \
  --cache-from "$cache_image" \
  --tag "${IMAGE}:${TAG}" \
  .
docker push "${IMAGE}:${TAG}"
