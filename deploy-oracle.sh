#!/bin/bash
# deploy-oracle.sh - One-command production deployment
if [ -z "$ORACLE_IP" ]; then
    echo "❌ Error: ORACLE_IP environment variable not set."
    exit 1
fi

echo "🚀 Deploying to Oracle Cloud ($ORACLE_IP)..."

ssh -o StrictHostKeyChecking=no ubuntu@$ORACLE_IP "
mkdir -p bharat-intel-quantum
cd bharat-intel-quantum &&
if [ ! -d .git ]; then
    echo 'Initializing workspace...'
fi
# git pull or rsync would go here in real CI
docker compose down &&
docker compose up -d --build &&
echo 'Stabilizing...' &&
sleep 30 &&
curl -s localhost:8000/quantum-health
"

echo "✅ Deployment complete."
