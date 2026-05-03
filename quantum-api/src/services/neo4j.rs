/// PRIORITY 2 — Neo4j Knowledge Graph Writes
/// Every intelligence synthesis creates ThreatEvent, Zone, Actor, and Provider nodes.
use neo4rs::{query, Graph};
use serde_json::Value;
use log::{error, info};

pub async fn connect(uri: &str, user: &str, pass: &str) -> Option<Graph> {
    match Graph::new(uri, user, pass).await {
        Ok(graph) => {
            let _ = initialize_schema(&graph).await;
            info!("[GRAPH] Neo4j connected: {}", uri);
            Some(graph)
        }
        Err(e) => {
            error!("[GRAPH] Neo4j connection failed: {}", e);
            None
        }
    }
}

pub async fn initialize_schema(
    graph: &Graph,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let queries = vec![
        "CREATE CONSTRAINT entity_name IF NOT EXISTS FOR (e:Entity) REQUIRE e.name IS UNIQUE",
        "CREATE CONSTRAINT article_url IF NOT EXISTS FOR (a:Article) REQUIRE a.url IS UNIQUE",
        "CREATE CONSTRAINT location_name IF NOT EXISTS FOR (l:Location) REQUIRE l.name IS UNIQUE",
        "CREATE CONSTRAINT vessel_mmsi IF NOT EXISTS FOR (v:Vessel) REQUIRE v.mmsi IS UNIQUE",
        "CREATE CONSTRAINT threat_event_id IF NOT EXISTS FOR (t:ThreatEvent) REQUIRE t.correlation_id IS UNIQUE",
        "CREATE CONSTRAINT zone_name IF NOT EXISTS FOR (z:Zone) REQUIRE z.name IS UNIQUE",
        "CREATE CONSTRAINT actor_name IF NOT EXISTS FOR (a:Actor) REQUIRE a.name IS UNIQUE",
        "CREATE CONSTRAINT provider_name IF NOT EXISTS FOR (p:Provider) REQUIRE p.name IS UNIQUE",
    ];

    for q in queries {
        let _ = graph.run(query(q)).await;
    }
    Ok(())
}

/// PRIORITY 2 — Write full intelligence synthesis to graph.
/// Creates ThreatEvent → Zone, Actor, Provider relationships.
pub async fn write_intelligence_synthesis(
    graph: Option<&Graph>,
    correlation_id: &str,
    query_str: &str,
    score: f64,
    level: &str,
    explanation: &str,
    key_actors: &[String],
    key_locations: &[String],
    sources_used: &Value,
    timestamp: &str,
) {
    let Some(g) = graph else {
        return;
    };

    // 1. Create ThreatEvent node
    let q = query(
        "MERGE (e:ThreatEvent {correlation_id: $id}) \
         SET e.query = $query, \
             e.risk_score = $score, \
             e.level = $level, \
             e.explanation = $explanation, \
             e.timestamp = $timestamp, \
             e.updated_at = datetime()",
    )
    .param("id", correlation_id.to_string())
    .param("query", query_str.to_string())
    .param("score", score)
    .param("level", level.to_string())
    .param("explanation", explanation.to_string())
    .param("timestamp", timestamp.to_string());

    match g.run(q).await {
        Ok(_) => info!("[GRAPH] ThreatEvent written: {}", correlation_id),
        Err(e) => {
            error!("[GRAPH] ThreatEvent write failed: {}", e);
            return;
        }
    }

    // 2. Create Zone nodes and LOCATED_IN relationships
    for location in key_locations {
        if location.trim().is_empty() {
            continue;
        }
        let q = query(
            "MERGE (z:Zone {name: $name}) \
             SET z.updated_at = datetime() \
             WITH z \
             MATCH (e:ThreatEvent {correlation_id: $id}) \
             MERGE (e)-[:LOCATED_IN]->(z)",
        )
        .param("name", location.clone())
        .param("id", correlation_id.to_string());

        if let Err(e) = g.run(q).await {
            error!("[GRAPH] Zone write failed for {}: {}", location, e);
        }
    }

    // 3. Create Actor nodes and INVOLVES relationships
    for actor in key_actors {
        if actor.trim().is_empty() {
            continue;
        }
        let q = query(
            "MERGE (a:Actor {name: $name}) \
             SET a.updated_at = datetime() \
             WITH a \
             MATCH (e:ThreatEvent {correlation_id: $id}) \
             MERGE (e)-[:INVOLVES]->(a)",
        )
        .param("name", actor.clone())
        .param("id", correlation_id.to_string());

        if let Err(e) = g.run(q).await {
            error!("[GRAPH] Actor write failed for {}: {}", actor, e);
        }
    }

    // 4. Create Provider nodes and USED_SOURCE relationships
    if let Some(obj) = sources_used.as_object() {
        for (provider, used) in obj {
            let contributed = match used {
                Value::Bool(b) => *b,
                Value::Number(n) => n.as_u64().unwrap_or(0) > 0,
                _ => false,
            };
            if !contributed {
                continue;
            }
            let q = query(
                "MERGE (p:Provider {name: $provider}) \
                 SET p.updated_at = datetime() \
                 WITH p \
                 MATCH (e:ThreatEvent {correlation_id: $id}) \
                 MERGE (e)-[:USED_SOURCE]->(p)",
            )
            .param("provider", provider.clone())
            .param("id", correlation_id.to_string());

            if let Err(e) = g.run(q).await {
                error!("[GRAPH] Provider write failed for {}: {}", provider, e);
            }
        }
    }

    // 5. Automatic Relationship Discovery: Link Actors to Zones mentioned in same assessment
    if !key_actors.is_empty() && !key_locations.is_empty() {
        for actor in key_actors {
            for location in key_locations {
                let q = query(
                    "MATCH (a:Actor {name: $actor_name}), (z:Zone {name: $zone_name}) \
                     MERGE (a)-[r:OPERATES_IN]->(z) \
                     SET r.last_seen = $ts, r.updated_at = datetime()",
                )
                .param("actor_name", actor.clone())
                .param("zone_name", location.clone())
                .param("ts", timestamp.to_string());
                let _ = g.run(q).await;
            }
        }
    }

    info!(
        "[GRAPH] Intelligence synthesis written: correlation_id={} level={} score={:.2}",
        correlation_id, level, score
    );
}

/// Legacy: upsert a simple entity-event pair.
pub async fn upsert_event(graph: Option<&Graph>, entity: &str, event_type: &str) {
    let Some(g) = graph else {
        return;
    };
    let q = query(
        "MERGE (e:Entity {name: $entity}) \
         MERGE (t:EventType {name: $event_type}) \
         MERGE (e)-[r:HAS_EVENT]->(t) \
         SET e.updated_at = datetime(), t.updated_at = datetime(), r.updated_at = datetime() \
         MERGE (evt:Event {entity: $entity, event_type: $event_type}) \
         SET evt.updated_at = datetime()",
    )
    .param("entity", entity.to_string())
    .param("event_type", event_type.to_string());
    let _ = g.run(q).await;
}

/// Store location-event relationship.
pub async fn upsert_location_event(
    graph: Option<&Graph>,
    location: &str,
    event_type: &str,
    event_data: &Value,
) {
    let Some(g) = graph else {
        return;
    };
    let q = query(
        "MERGE (loc:Location {name: $location}) \
         SET loc.lat = $lat, loc.lon = $lon, loc.updated_at = datetime() \
         MERGE (evt:Event {type: $event_type, location: $location}) \
         SET evt.data = $data, evt.updated_at = datetime() \
         MERGE (loc)-[r:HAS_EVENT]->(evt) \
         SET r.updated_at = datetime()",
    )
    .param("location", location.to_string())
    .param("event_type", event_type.to_string())
    .param(
        "lat",
        event_data
            .get("lat")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
    )
    .param(
        "lon",
        event_data
            .get("lon")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
    )
    .param("data", event_data.to_string());
    let _ = g.run(q).await;
}

/// Store vessel-route relationship.
pub async fn upsert_vessel_route(
    graph: Option<&Graph>,
    vessel_id: &str,
    vessel_name: &str,
    route_data: &Value,
) {
    let Some(g) = graph else {
        return;
    };
    let q = query(
        "MERGE (v:Vessel {mmsi: $vessel_id}) \
         SET v.name = $vessel_name, v.updated_at = datetime() \
         MERGE (r:Route {vessel: $vessel_id, timestamp: datetime()}) \
         SET r.path = $path, r.anomaly_score = $anomaly_score, r.updated_at = datetime() \
         MERGE (v)-[rel:FOLLOWS]->(r) \
         SET rel.updated_at = datetime()",
    )
    .param("vessel_id", vessel_id.to_string())
    .param("vessel_name", vessel_name.to_string())
    .param(
        "path",
        route_data
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown"),
    )
    .param(
        "anomaly_score",
        route_data
            .get("anomaly_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
    );
    let _ = g.run(q).await;
}

/// Store entity-threat relationship.
pub async fn upsert_entity_threat(
    graph: Option<&Graph>,
    entity_name: &str,
    entity_type: &str,
    threat_data: &Value,
) {
    let Some(g) = graph else {
        return;
    };
    let q = query(
        "MERGE (e:Entity {name: $entity_name}) \
         SET e.type = $entity_type, e.updated_at = datetime() \
         MERGE (t:Threat {entity: $entity_name, level: $threat_level}) \
         SET t.score = $threat_score, t.data = $data, t.updated_at = datetime() \
         MERGE (e)-[r:POSES_THREAT]->(t) \
         SET r.severity = $threat_score, r.updated_at = datetime()",
    )
    .param("entity_name", entity_name.to_string())
    .param("entity_type", entity_type.to_string())
    .param(
        "threat_level",
        threat_data
            .get("level")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown"),
    )
    .param(
        "threat_score",
        threat_data
            .get("score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
    )
    .param("data", threat_data.to_string());
    let _ = g.run(q).await;
}

/// PRIORITY 5 — Write vessel to graph with dark vessel support.
pub async fn upsert_vessel(
    graph: Option<&Graph>,
    mmsi: &str,
    name: &str,
    lat: f64,
    lon: f64,
    risk_score: f64,
    is_dark: bool,
    timestamp: &str,
) {
    let Some(g) = graph else {
        return;
    };
    let q = query(
        "MERGE (v:Vessel {mmsi: $mmsi}) \
         SET v.name = $name, \
             v.last_lat = $lat, \
             v.last_lon = $lon, \
             v.risk_score = $risk_score, \
             v.is_dark = $is_dark, \
             v.last_seen = $timestamp, \
             v.updated_at = datetime()",
    )
    .param("mmsi", mmsi.to_string())
    .param("name", name.to_string())
    .param("lat", lat)
    .param("lon", lon)
    .param("risk_score", risk_score)
    .param("is_dark", is_dark)
    .param("timestamp", timestamp.to_string());

    if let Err(e) = g.run(q).await {
        error!("[GRAPH] Vessel write failed for {}: {}", mmsi, e);
    }
}

/// PRIORITY 5 — Write dark vessel event to graph.
pub async fn upsert_dark_vessel_event(
    graph: Option<&Graph>,
    mmsi: &str,
    zone: &str,
    last_seen: &str,
    timestamp: &str,
) {
    let Some(g) = graph else {
        return;
    };
    let q = query(
        "MERGE (v:Vessel {mmsi: $mmsi}) \
         CREATE (d:DarkVesselEvent { \
             mmsi: $mmsi, \
             zone: $zone, \
             last_seen: $last_seen, \
             detected_at: $timestamp \
         }) \
         MERGE (v)-[:GENERATED_ALERT]->(d) \
         WITH v, d \
         MERGE (z:Zone {name: $zone}) \
         MERGE (v)-[:LAST_SEEN_IN]->(z)",
    )
    .param("mmsi", mmsi.to_string())
    .param("zone", zone.to_string())
    .param("last_seen", last_seen.to_string())
    .param("timestamp", timestamp.to_string());

    if let Err(e) = g.run(q).await {
        error!("[GRAPH] DarkVesselEvent write failed for {}: {}", mmsi, e);
    } else {
        info!("[GRAPH] DarkVesselEvent written for MMSI {}", mmsi);
    }
}

/// Query related threats by location.
pub async fn get_related_threats(graph: Option<&Graph>, location: &str) -> Vec<Value> {
    let Some(g) = graph else {
        return vec![];
    };
    let q = query(
        "MATCH (loc:Location {name: $location})-[r:HAS_EVENT]->(evt:Event) \
         RETURN loc.name as location, evt.type as event_type, evt.data as data \
         ORDER BY evt.updated_at DESC LIMIT 10",
    )
    .param("location", location.to_string());

    let mut results = Vec::new();
    match g.execute(q).await {
        Ok(mut stream) => {
            while let Ok(Some(row)) = stream.next().await {
                let mut obj = serde_json::Map::new();
                if let Ok(loc) = row.get::<String>("location") {
                    obj.insert("location".to_string(), Value::String(loc));
                }
                if let Ok(evt_type) = row.get::<String>("event_type") {
                    obj.insert("event_type".to_string(), Value::String(evt_type));
                }
                if let Ok(data) = row.get::<String>("data") {
                    obj.insert("data".to_string(), Value::String(data));
                }
                results.push(Value::Object(obj));
            }
        }
        Err(e) => error!("[GRAPH] Query failed: {}", e),
    }
    results
}

/// PRIORITY 2 — Export graph data for D3.js visualization.
/// Returns a JSON structure with nodes and links.
pub async fn export_graph_data(graph: Option<&Graph>) -> Value {
    let Some(g) = graph else {
        return serde_json::json!({"nodes": [], "links": []});
    };

    let mut nodes = Vec::new();
    let mut links = Vec::new();
    let mut seen_nodes = std::collections::HashSet::new();

    // Query for ThreatEvents and their relationships
    let q = query(
        "MATCH (e:ThreatEvent) \
         OPTIONAL MATCH (e)-[r]->(target) \
         RETURN e, type(r) as rel_type, target LIMIT 100"
    );

    match g.execute(q).await {
        Ok(mut stream) => {
            while let Ok(Some(row)) = stream.next().await {
                // Process ThreatEvent node
                if let Ok(e) = row.get::<neo4rs::Node>("e") {
                    let id = e.get::<String>("correlation_id").unwrap_or_default();
                    if !seen_nodes.contains(&id) {
                        nodes.push(serde_json::json!({
                            "id": id,
                            "label": "ThreatEvent",
                            "type": "event",
                            "query": e.get::<String>("query").unwrap_or_default(),
                            "risk_score": e.get::<f64>("risk_score").unwrap_or(0.0),
                            "level": e.get::<String>("level").unwrap_or_default()
                        }));
                        seen_nodes.insert(id.clone());
                    }

                    // Process target node and relationship
                    if let Ok(rel_type) = row.get::<String>("rel_type") {
                        if let Ok(target) = row.get::<neo4rs::Node>("target") {
                            let labels = target.labels();
                            let target_label = labels.first().map(|s| s.to_string()).unwrap_or_else(|| "Unknown".to_string());
                            
                            let target_name = if let Ok(n) = target.get::<String>("name") {
                                n
                            } else if let Ok(m) = target.get::<String>("mmsi") {
                                m
                            } else {
                                "Unknown".to_string()
                            };
                            
                            let target_id = format!("{}:{}", target_label, target_name);
                            
                            if !seen_nodes.contains(&target_id) {
                                nodes.push(serde_json::json!({
                                    "id": target_id,
                                    "label": target_label,
                                    "type": target_label.to_lowercase(),
                                    "name": target_name
                                }));
                                seen_nodes.insert(target_id.clone());
                            }

                            links.push(serde_json::json!({
                                "source": id,
                                "target": target_id,
                                "label": rel_type
                            }));
                        }
                    }
                }
            }
        }
        Err(e) => error!("[GRAPH] Export failed: {}", e),
    }

    serde_json::json!({
        "nodes": nodes,
        "edges": links
    })
}
