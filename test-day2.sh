#!/bin/bash
set -e
echo "🚀 Starting Bharat Intel Quantum Day 2 Tests..."

# Restart to apply changes
docker compose restart quantum-api
echo "⏳ Waiting for API to restart (45s)..."
sleep 45

echo "🔍 Testing Dawn RSS Ingest..."
# Note: /ingest-pakistan is POST
curl -s -X POST localhost:8000/ingest-pakistan | jq -r '.[].sources[]' | grep -q "Dawn" || { echo "  ❌ Dawn RSS check: FAILED"; exit 1; }
echo "  ✅ Dawn RSS: PASSED"

echo "🔍 Testing Pakistan threats retrieval..."
curl -s localhost:8000/pakistan-threats | jq -r '.[0].actor' | grep -Eq "Saeed|JeM" || { echo "  ❌ Pakistan Threats check: FAILED"; exit 1; }
echo "  ✅ Pakistan Threats: PASSED (Found Saeed/JeM)"

echo "🔍 Verifying Quantum security remains active..."
curl -s localhost:8000/quantum-health | jq -r '.kyber1024' | grep -q "active" || { echo "  ❌ Quantum Health check: FAILED"; exit 1; }
echo "  ✅ Quantum Kyber1024: ACTIVE"

echo "----------------------------------------"
echo "DAY 2 PAKISTAN X INGESTER LIVE ✅"
docker compose ps | grep quantum-api
