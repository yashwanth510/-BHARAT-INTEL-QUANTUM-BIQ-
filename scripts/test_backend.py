#!/usr/bin/env python3
"""Isolated native API/Redis/WebSocket integration tests. No external providers.
Requires Python websockets (pip install websockets). Does not touch the user's Redis.
"""
import asyncio
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
import websockets

ROOT = Path(__file__).resolve().parents[1]

def port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]

async def main():
    redis_port, api_port = port(), port()
    base = f'http://127.0.0.1:{api_port}'
    token = secrets.token_urlsafe(32)
    env = os.environ.copy()
    env.update({key:'' for key in ['OPENSKY_CLIENT_ID','OPENSKY_CLIENT_SECRET','AISSTREAM_API_KEY','GPS_API_KEY','GPS_API_BASE_URL','OPENWEATHER_API_KEY','SENTINEL_CLIENT_ID','SENTINEL_CLIENT_SECRET','TAVILY_API_KEY','GEOAPIFY_API_KEY']})
    env.update(PORT=str(api_port), REDIS_URL=f'redis://127.0.0.1:{redis_port}', INGEST_ENABLED='false', ADMIN_API_TOKEN=token, GPS_WEBHOOK_TOKEN=token, RUST_LOG='error')
    def request(path, method='GET', data=None, auth=False, headers=None):
        req_headers={'Content-Type':'application/json', **(headers or {})}
        if auth: req_headers['Authorization']='Bearer '+token
        req=urllib.request.Request(base+path,method=method,headers=req_headers,data=json.dumps(data).encode() if data is not None else None)
        try: result=urllib.request.urlopen(req,timeout=8)
        except urllib.error.HTTPError as error: result=error
        body=result.read()
        return result.status, json.loads(body) if body else None, result.headers
    with tempfile.TemporaryDirectory(prefix='biq-test-') as tmp, open(os.devnull,'w') as log:
        redis_process=subprocess.Popen(['redis-server','--port',str(redis_port),'--bind','127.0.0.1','--save','','--appendonly','no','--dir',tmp],stdout=log,stderr=log)
        backend=None
        try:
            time.sleep(.4)
            backend=subprocess.Popen([str(Path(os.environ.get('BIQ_BACKEND_BINARY', str(ROOT/'backend/target/debug/biq-backend'))).resolve())],cwd=ROOT,env=env,stdout=log,stderr=log)
            for _ in range(80):
                try:
                    if request('/health')[0]==200: break
                except OSError: pass
                await asyncio.sleep(.1)
            else: raise AssertionError('Backend did not start')
            for path in ['/api/flights','/api/vessels','/api/vehicles','/api/anomalies']:
                status,data,_=request(path); assert status==200 and data==[], (path,status,data)
            assert len(request('/api/borders')[1]['features'])>10
            assert request('/api/allowlist')[0]==401
            assert request('/ingest/gps','POST',{'id':'test','lat':22,'lon':80})[0]==401
            assert request('/api/allowlist','PUT',{'kind':'invalid','id':'test'},True)[0]==400
            assert request('/ingest/gps','POST',{'id':'test','lat':999,'lon':80},True)[0]==400
            assert request('/api/weather?lat=NaN&lon=80')[0]==400
            assert request('/api/weather?lat=22&lon=80')[0]==503
            for path in ['/api/satellite/snapshot','/api/osint/enrich']:
                assert request(path,'POST',{'lat':999,'lon':80})[0]==400
                assert request(path,'POST',{'lat':22,'lon':80})[0]==503
            assert request('/api/satellite/snapshot','POST',{'lat':22,'lon':80,'bbox_deg':-1})[0]==400
            assert request('/api/location?lat=22&lon=80')[0]==503
            _,_,headers=request('/api/flights',headers={'Origin':'http://localhost:3000'})
            assert headers.get('Access-Control-Allow-Origin')=='http://localhost:3000'
            _,_,headers=request('/api/flights',headers={'Origin':'https://untrusted.example'})
            assert not headers.get('Access-Control-Allow-Origin')
            async with websockets.connect(f'ws://127.0.0.1:{api_port}/ws/live') as ws:
                assert json.loads(await asyncio.wait_for(ws.recv(),5))['type']=='heartbeat'
                position={'device_id':'integration-test','lat':23.0,'lon':68.1,'speed':10}
                assert request('/ingest/gps','POST',position,True)[0]==204
                received=[]
                while not any(item['type']=='vehicle_update' for item in received):
                    received.append(json.loads(await asyncio.wait_for(ws.recv(),5)))
                assert any(item['type']=='anomaly' for item in received)
                update=next(item for item in received if item['type']=='vehicle_update')
                assert update['lat']==position['lat'] and update['lon']==position['lon']
                assert request('/api/vehicles')[1][0]['device_id']=='integration-test'
                count=len(request('/api/anomalies')[1]); assert count>0
                assert request('/ingest/gps','POST',position,True)[0]==204
                assert len(request('/api/anomalies')[1])==count
                assert request('/api/allowlist','PUT',{'kind':'vehicle','id':'trusted-test'},True)[0]==200
                assert request('/api/allowlist',auth=True)[1][0]['id']=='trusted-test'
                position['device_id']='trusted-test'
                assert request('/ingest/gps','POST',position,True)[0]==204
                assert len(request('/api/anomalies')[1])==count
            redis_process.terminate(); redis_process.wait(timeout=5)
            assert request('/health')[0]==503
            assert request('/api/vehicles')[0]==503
            print('PASS: health, Redis failure, 11 REST routes, auth, CORS, validation, disabled providers, GPS → Redis → WebSocket, anomaly dedupe, allowlist suppression')
        finally:
            for process in [backend,redis_process]:
                if process and process.poll() is None:
                    process.terminate()
                    try: process.wait(timeout=5)
                    except subprocess.TimeoutExpired: process.kill(); process.wait()

if __name__=='__main__': asyncio.run(main())
