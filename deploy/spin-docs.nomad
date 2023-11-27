variable "region" {
  type    = string
}

variable "ecr_ref" {
  type        = string
  description = "The ECR reference of the Spin app for the Spin Docs website"
}

variable "commit_sha" {
  type        = string
  description = "The git commit sha that the website is published from"
}

job "spin-docs" {
  type = "service"
  datacenters = [
    "${var.region}a",
    "${var.region}b",
    "${var.region}c",
    "${var.region}d",
    "${var.region}e",
    "${var.region}f"
  ]

  # Add unique metadata to support recreating the job even if var.ecr_ref
  # represents a mutable tag (eg latest).
  meta {
    commit_sha = var.commit_sha
  }

  group "spin-docs" {
    count = 3

    update {
      max_parallel      = 1
      canary            = 3
      min_healthy_time  = "10s"
      healthy_deadline  = "10m"
      progress_deadline = "15m"
      auto_revert       = true
      auto_promote      = true
    }

    network {
      port "http" {}
    }

    service {
      name = "spin-docs-${NOMAD_NAMESPACE}"
      port = "http"

      tags = [
        "traefik.enable=true",
        "traefik.http.routers.spin-docs-${NOMAD_NAMESPACE}.rule=Host(`spin.fermyon.dev`)",
        "traefik.http.routers.spin-docs-${NOMAD_NAMESPACE}.entryPoints=websecure",
        "traefik.http.routers.spin-docs-${NOMAD_NAMESPACE}.tls=true",
        "traefik.http.routers.spin-docs-${NOMAD_NAMESPACE}.tls.certresolver=letsencrypt-cf-prod",
        "traefik.http.routers.spin-docs-${NOMAD_NAMESPACE}.tls.domains[0].main=spin.fermyon.dev"
      ]

      check {
        type     = "http"
        path     = "/.well-known/spin/health"
        interval = "10s"
        timeout  = "2s"
      }
    }

    task "server" {
      driver = "exec"

      vault {
        policies = ["svc-website-runner"]
      }

      artifact {
        source = "https://github.com/fermyon/spin/releases/download/v2.0.1/spin-v2.0.1-linux-amd64.tar.gz"
        options {
          checksum = "sha256:686bb12b9244ed33bf54a53e62303879036632b476ad09a728172b260f26c8e7"
        }
      }

      env {
        RUST_LOG   = "spin=trace"
        BASE_URL   = "https://spin.fermyon.dev"
      }

      config {
        command = "${NOMAD_TASK_DIR}/run.sh"
      }

      template {
        destination = "${NOMAD_TASK_DIR}/run.sh"
        change_mode = "restart"
        data = <<-EOF
        #!/bin/bash
        set -euo pipefail

        IFS=/ read -r registry image <<< "${var.ecr_ref}"
        aws ecr get-login-password --region ${var.region} | \
          ${NOMAD_TASK_DIR}/spin registry login --username AWS --password-stdin $registry

        ${NOMAD_TASK_DIR}/spin up \
          --from-registry ${var.ecr_ref} \
          --listen ${NOMAD_IP_http}:${NOMAD_PORT_http} \
          --log-dir ${NOMAD_ALLOC_DIR}/logs \
          --temp ${NOMAD_ALLOC_DIR}/tmp \
          -e BASE_URL=${BASE_URL}
        EOF
      }

      template {
        destination = "${NOMAD_SECRETS_DIR}/env.txt"
        change_mode = "noop"
        env         = true
        data        = <<-EOF
        {{ with secret "aws/creds/website-runner" "ttl=15m" }}
        AWS_ACCESS_KEY_ID="{{ .Data.access_key }}"
        AWS_SECRET_ACCESS_KEY="{{ .Data.secret_key }}"
        AWS_SESSION_TOKEN="{{ .Data.security_token }}"
        {{ end }}
        EOF
      }
    }
  }
}
