# BHARAT INTEL QUANTUM (BIQ)

**iDEX ADITI4 - Quantum-Secure Intelligence Platform**
*National Security & Intelligence Analytics at Global Scale*

## Project Overview
BIQ is a next-generation intelligence platform designed for the NIA, RAW, and Indian Army. It provides a quantum-secure infrastructure for global signal fusion, link analysis, and cognitive threat prediction.

## Day 1: Foundation Setup
- **Rust backend**: High-performance Actix-web server.
- **Quantum Security**: Kyber1024 (NIST PQC) key encapsulation mechanism.
- **Data Layers**: 
  - Neo4j (Graph GNN)
  - Kafka (Global data ingestion)
  - Redis (Real-time caching)
- **Infrastructure**: Dockerized multi-service architecture ready for Oracle Cloud.

## Getting Started
1. **Initialize Environment**:
   ```bash
   cp .env.example .env
   ```
2. **Launch & Test**:
   ```bash
   chmod +x test.sh
   ./test.sh
   ```

## API Specifications
- `GET /quantum-health`: Verifies Kyber1024 engine.
- `GET /health`: System status overview.
- `POST /ingest-threat`: Encrypts/Decrypts threat signals using PQC.

## Security Architecture
BIQ utilizes a 10-layer defense strategy (L0-L9), from global data collection (L0) to TLA+ formal proofs (L9).

---
**Status**: DAY 1 COMPLETE ✅  
**Deadline**: May 4, 5PM IST
