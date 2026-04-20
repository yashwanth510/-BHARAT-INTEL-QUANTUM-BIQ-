#!/bin/bash
# test.sh - FULL auto-test
echo "🚀 Starting Bharat Intel Quantum Day 1 Tests..."

docker compose down -v
echo "📦 Building and starting services..."
docker compose up -d --build

echo "⏳ Waiting for services to stabilize (60s)..."
# Using a loop to check health instead of just sleeping
COUNT=0
MAX=12
while [ $COUNT -lt $MAX ]; do
    if [ $(docker compose ps | grep -c "healthy") -ge 3 ]; then
        echo "✅ Core services healthy!"
        break
    fi
    sleep 5
    COUNT=$((COUNT+1))
    echo "Still waiting... ($((COUNT*5))s)"
done

echo "🔍 Testing quantum-api..."
# Test quantum
HEALTH_QR=$(curl -s localhost:8000/quantum-health)
echo $HEALTH_QR | jq -r '.kyber1024' | grep -q "active"
if [ $? -eq 0 ]; then
    echo "  ✅ Kyber1024: ACTIVE"
    echo "  🔑 Public Key: $(echo $HEALTH_QR | jq -r '.public_key')"
else
    echo "  ❌ Kyber1024: FAILED"
    exit 1
fi

# Test health
curl -s localhost:8000/health | jq -r '.services' | grep -q "4" || { echo "  ❌ Health Services check: FAILED"; exit 1; }
echo "  ✅ Health Services: 4 READY"

# Test neo4j
curl -s http://localhost:7474 | grep -q "neo4j_version" || { echo "  ❌ Neo4j: FAILED"; exit 1; }
echo "  ✅ Neo4j: ACCESSIBLE"

echo "----------------------------------------"
echo "DAY 1 ALL TESTS PASSED ✅"
docker compose ps
