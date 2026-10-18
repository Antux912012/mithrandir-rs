#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${HOME}/.env"

if [ ! -f "${ENV_FILE}" ]; then
  echo "Error: ~/.env file not found."
  echo "Please store your token first using the provided command."
  exit 1
fi

# Extract GITHUB_TOKEN safely without printing
GITHUB_TOKEN=$(grep -E "^GITHUB_TOKEN=" "${ENV_FILE}" | cut -d'=' -f2- | tr -d '\r\n"')

if [ -z "${GITHUB_TOKEN}" ]; then
  echo "Error: GITHUB_TOKEN not found in ~/.env."
  exit 1
fi

echo "==> Verifying GitHub token authentication..."
USER_RESP=$(curl -s -H "Authorization: Bearer ${GITHUB_TOKEN}" \
                 -H "Accept: application/vnd.github.v3+json" \
                 https://api.github.com/user)

USER_LOGIN=$(echo "${USER_RESP}" | grep -o '"login": "[^"]*' | cut -d'"' -f4 || true)

if [ -z "${USER_LOGIN}" ]; then
  echo "Error: GitHub authentication failed. Please check your token permissions (repo scope required)."
  exit 1
fi

echo "==> Authenticated as GitHub user: ${USER_LOGIN}"

REPO_NAME="mithrandir-rs"
REPO_DESC="Modern GNU/Linux Cleaner, Optimizer, Telemetry & Diagnostics in Rust (GTK4/Libadwaita)"

echo "==> Checking if repository ${USER_LOGIN}/${REPO_NAME} exists..."
CHECK_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer ${GITHUB_TOKEN}" \
  -H "Accept: application/vnd.github.v3+json" \
  "https://api.github.com/repos/${USER_LOGIN}/${REPO_NAME}")

if [ "${CHECK_STATUS}" -eq 404 ]; then
  echo "==> Creating public repository ${USER_LOGIN}/${REPO_NAME} on GitHub..."
  CREATE_RESP=$(curl -s -X POST \
    -H "Authorization: Bearer ${GITHUB_TOKEN}" \
    -H "Accept: application/vnd.github.v3+json" \
    https://api.github.com/user/repos \
    -d "{\"name\":\"${REPO_NAME}\",\"description\":\"${REPO_DESC}\",\"private\":false,\"has_issues\":true,\"has_projects\":true,\"has_wiki\":true}")
  echo "==> Repository created successfully."
elif [ "${CHECK_STATUS}" -eq 200 ]; then
  echo "==> Repository ${USER_LOGIN}/${REPO_NAME} already exists on GitHub."
else
  echo "==> GitHub API returned status ${CHECK_STATUS}."
fi

# Configure git remote
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

git branch -M main

REMOTE_URL="https://github.com/${USER_LOGIN}/${REPO_NAME}.git"
if git remote | grep -q "^origin$"; then
  git remote set-url origin "${REMOTE_URL}"
else
  git remote add origin "${REMOTE_URL}"
fi

echo "==> Pushing 'main' branch to ${REMOTE_URL}..."
# Use in-memory credential push to prevent storing token in .git/config
git push "https://x-access-token:${GITHUB_TOKEN}@github.com/${USER_LOGIN}/${REPO_NAME}.git" main:main -u
git config branch.main.remote origin
git fetch origin

echo "==> Successfully published to: https://github.com/${USER_LOGIN}/${REPO_NAME}"
