#!/usr/bin/env python3
import psycopg2
import select
import time

# Connect and listen
conn = psycopg2.connect(host='localhost', port=5432, database='market_data', user='daws')
conn.set_isolation_level(psycopg2.extensions.ISOLATION_LEVEL_AUTOCOMMIT)
cur = conn.cursor()
cur.execute('LISTEN new_trade;')
print('Listening for PostgreSQL NOTIFY events...')

# Check for notifications for 5 seconds
for i in range(5):
    if select.select([conn], [], [], 1) != ([], [], []):
        conn.poll()
        while conn.notifies:
            notify = conn.notifies.pop(0)
            print(f'📡 NOTIFY received: {notify.payload}')
    else:
        print(f'⏳ Waiting... ({i+1}/5)')
    time.sleep(1)

print('Test completed')