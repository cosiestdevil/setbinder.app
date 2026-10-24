"""Exercise cold-cache and failure paths without registry credentials or Docker."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).with_name('build-image.sh')
IMAGE = 'europe-west2-docker.pkg.dev/example/services/setbinder'


class BuildPipelineTests(unittest.TestCase):
    def run_build(self, fail='', image=IMAGE, tag='abc123'):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            log = root / 'commands.jsonl'
            docker = root / 'docker'
            docker.write_text('''#!/usr/bin/env python3
import json, os, sys
with open(os.environ['MOCK_DOCKER_LOG'], 'a') as log:
    log.write(json.dumps(sys.argv[1:]) + '\\n')
sys.exit(1 if sys.argv[1] == os.environ.get('MOCK_DOCKER_FAIL') else 0)
''')
            docker.chmod(0o755)
            env = dict(os.environ, PATH=f"{root}:{os.environ['PATH']}",
                       MOCK_DOCKER_LOG=str(log), MOCK_DOCKER_FAIL=fail,
                       IMAGE=image, TAG=tag)
            result = subprocess.run(['bash', str(SCRIPT)], env=env,
                                    capture_output=True, text=True)
            calls = [json.loads(line) for line in log.read_text().splitlines()] if log.exists() else []
            return result, calls

    def test_warm_build_reuses_dependency_image_and_pushes_both_targets(self):
        result, calls = self.run_build()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([call[0] for call in calls], ['pull', 'build', 'push', 'build', 'push'])
        self.assertEqual(calls[0][1], f'{IMAGE}:build-deps')
        self.assertIn('dependencies', calls[1])
        self.assertIn(f'{IMAGE}:build-deps', calls[3])
        self.assertEqual(calls[-1], ['push', f'{IMAGE}:abc123'])

    def test_missing_cache_allows_a_cold_build(self):
        result, calls = self.run_build(fail='pull')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('building dependencies from scratch', result.stderr)
        self.assertEqual(calls[-1], ['push', f'{IMAGE}:abc123'])

    def test_compile_failure_stops_before_any_push(self):
        result, calls = self.run_build(fail='build')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual([call[0] for call in calls], ['pull', 'build'])

    def test_cache_push_failure_prevents_application_publish(self):
        result, calls = self.run_build(fail='push')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual([call[0] for call in calls], ['pull', 'build', 'push'])

    def test_invalid_configuration_never_calls_docker(self):
        for image, tag in [('', 'abc123'), ('bad/image', 'abc123'),
                           (IMAGE + ':latest', 'abc123'), (IMAGE, ''),
                           (IMAGE, 'bad tag')]:
            with self.subTest(image=image, tag=tag):
                result, calls = self.run_build(image=image, tag=tag)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(calls, [])


if __name__ == '__main__':
    unittest.main()
