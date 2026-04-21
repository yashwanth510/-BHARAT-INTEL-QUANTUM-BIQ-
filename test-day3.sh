#!/bin/bash
set -e
echo "🚀 Starting Bharat Intel Quantum Day 3 Tests..."

# Restart to apply changes
docker compose restart quantum-api
echo "⏳ Waiting for API to restart (45s)..."
sleep 45

echo "🔍 Testing China Ingest (Global Times)..."
curl -s -X POST localhost:8000/ingest-china | jq -r '.[].sources[]' | grep -q "Global Times" || { echo "  ❌ China Ingest check: FAILED"; exit 1; }
echo "  ✅ China Ingest: PASSED"

echo "🔍 Testing China threats retrieval..."
curl -s localhost:8000/china-threats | jq 'length' | grep -q "1" || { echo "  ❌ China Threats check: FAILED"; exit 1; }
echo "  ✅ China Threats: PASSED"

echo "🔍 Testing Prediction engine..."
curl -s localhost:8000/predict | jq '.likelihood' | grep -q "87" || { echo "  ❌ Prediction check: FAILED"; exit 1; }
echo "  ✅ Prediction: 87% Likelihood"

echo "🔍 Verifying Pakistan threats (Day 2 persistence)..."
curl -s localhost:8000/pakistan-threats | jq 'length' | grep -q "1" || { echo "  ❌ Pakistan Threats check: FAILED"; exit 1; }
echo "  ✅ Pakistan Threats: PASSED"

echo "🔍 Verifying Quantum security (Day 1 integrity)..."
curl -s localhost:8000/quantum-health | jq -r '.kyber1024' | grep -q "active" || { echo "  ❌ Quantum Health check: FAILED"; exit 1; }
echo "  ✅ Quantum Kyber1024: ACTIVE"

echo "----------------------------------------"
echo "DAY 3 CHINA + PREDICTION LIVE ✅"
docker compose ps | grep quantum-api
