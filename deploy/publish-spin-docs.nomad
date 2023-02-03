variable "region" {
  type    = string
}

variable "git_ref" {
  type        = string
  default     = "refs/heads/main"
  description = "Git ref to use for the spin repo clone. Default: refs/heads/main"
}

variable "commit_sha" {
  type        = string
  default     = ""
  description = "Specific commit SHA to check out. Default: empty/none"
}

job "publish-spin-docs" {
  type        = "batch"
  datacenters = [
    "${var.region}a",
    "${var.region}b",
    "${var.region}c",
    "${var.region}d",
    "${var.region}e",
    "${var.region}f"
  ]

  group "publish-spin-docs" {
    count = 1

    task "publish-spin-docs" {
      driver = "exec"

      artifact {
        source = "https://github.com/fermyon/spin/releases/download/v0.8.0/spin-v0.8.0-linux-amd64.tar.gz"
        options {
          checksum = "sha256:0ef31fe6e2b4d34ddd089b01a1f88820f88c456276bfe4e1477836a6087654c1"
        }
      }

      env {
        BINDLE_URL = "http://bindle.service.consul:3030/v1"
      }

      template {
        data = <<-EOF
        #!/bin/bash
        set -euo pipefail

        readonly repo_dir="${NOMAD_ALLOC_DIR}/spin"

        # Capture branch/tag name from full ref
        readonly branch="$(echo ${var.git_ref} | cut -d'/' -f3-)"
        
        # Directory and contents may be non-empty if this job is retrying while the
        # bindle server is still coming up. (git clone will complain)
        rm -rf ${repo_dir}
        git clone -b ${branch} https://github.com/fermyon/spin.git ${repo_dir}
        cd ${repo_dir}/docs

        # Check out commit if provided
        [[ "${var.commit_sha}" == "" ]] || git checkout ${var.commit_sha}

        ${NOMAD_TASK_DIR}/spin bindle push \
          -f spin.toml \
          -d ./staging_dir \
          --buildinfo "g$(git rev-parse HEAD)-$(date '+%Y%m%d%M%H%M%S')"
        EOF
        destination = "${NOMAD_TASK_DIR}/publish.bash"
        perms       = "700"
      }

      config {
        command = "${NOMAD_TASK_DIR}/publish.bash"
      }
    }
  }
}