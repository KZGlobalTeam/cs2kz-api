# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (C) zer0.k, AlphaKeks <alphakeks@dawn.sh>

import common
import json
import mariadb
import numpy as np
import os
import sys
import time
import traceback

from scipy import stats
from typing import Any, List, Tuple
from urllib.parse import urlparse

def warn(msg):
    sys.stderr.write(json.dumps({'warning': msg}) + '\n')

def open_database_conn():
    DATABASE_URL = os.getenv('DATABASE_URL', 'mysql://schnose:csgo-kz-is-dead-boys@127.0.0.1:3306/cs2kz')

    database_url = urlparse(DATABASE_URL)
    return mariadb.connect(
        user = database_url.username,
        password = database_url.password,
        host = database_url.hostname,
        port = database_url.port or 3306,
        database = database_url.path.lstrip('/'),
        reconnect = True
    )

def process_input(database_conn, line):
    """
    Processes a single line read from stdin.

    The line is expected to contain a JSON object with the following keys:
        * `filter_id` - ID of the filter to calculate

    An example object could look like this:
        ```json
        {
          "filter_id": 74
        }
        ```

    The function will write a single line response to stdout.

    That line is a JSON object with the following keys:
        * `filter_id` - the ID of the calculated filter
        * `timings` - an object containing timings of the individual operations
                      performed

    The `timings` object will contain the following keys:
        * `db_query_ms` - the time it took to query the database for the
                          required information, in milliseconds, as a floating
                          point number
        * `nub_fit_ms` - the time it took to fit the NUB distribution, in
                         milliseconds, as a floating point number
        * `nub_compute_ms` - the time it took to calculate new points for the
                             NUB leaderboard, in milliseconds, as a floating
                             point number
        * `pro_fit_ms` - the time it took to fit the PRO distribution, in
                         milliseconds, as a floating point number
        * `pro_compute_ms` - the time it took to calculate new points for the
                             PRO leaderboard, in milliseconds, as a floating
                             point number
        * `db_write_ms` - the time it took to write everything back to the
                          database, in milliseconds, as a floating point number
    """

    cursor = database_conn.cursor()

    response = {
        'filter_id': None,
        'timings': {
            'db_query_ms': 0.0,
            'nub_fit_ms': 0.0,
            'nub_compute_ms': 0.0,
            'pro_fit_ms': 0.0,
            'pro_compute_ms': 0.0,
            'db_write_ms': 0.0
        }
    }
    data = json.loads(line)
    filter_id = data['filter_id']
    response['filter_id'] = filter_id

    start = time.time()
    cursor.execute("""
        SELECT
            bnr.record_id,
            bnr.time,
            bnr.points
        FROM
            BestNubRecords AS bnr
        WHERE
            bnr.filter_id = ?
        ORDER BY
            bnr.time ASC
    """, (
        filter_id,
    ))
    nub_records: List[Tuple[Any, float, float]] = cursor.fetchall()
    cursor.execute("""
        SELECT
            bpr.record_id,
            bpr.time,
            bpr.points
        FROM
            BestProRecords AS bpr
        WHERE
            bpr.filter_id = ?
        ORDER BY
            bpr.time ASC
    """, (
        filter_id,
    ))
    pro_records: List[Tuple[Any, float, float]] = cursor.fetchall()
    cursor.execute("""
        SELECT
            cf.nub_tier,
            cf.pro_tier
        FROM
            CourseFilters cf
        WHERE
            cf.id = ?
    """, (
        filter_id,
    ))
    filter_row = cursor.fetchone()

    # Fetch previous distribution parameters for warm start
    # (both nub and pro in one query)
    cursor.execute("""
        SELECT
            is_pro_leaderboard,
            a,
            b,
            loc,
            scale,
            top_scale
        FROM
            PointDistributionData
        WHERE
            filter_id = ?
        ORDER BY
            is_pro_leaderboard
    """, (
        filter_id,
    ))
    dist_params_rows = cursor.fetchall()

    prev_nub_params = None
    prev_pro_params = None
    for row in dist_params_rows:
        if row[0] == 0:  # is_pro_leaderboard = 0 (nub)
            prev_nub_params = (row[1], row[2], row[3], row[4], row[5])
        elif row[0] == 1:  # is_pro_leaderboard = 1 (pro)
            prev_pro_params = (row[1], row[2], row[3], row[4], row[5])

    response['timings']['db_query_ms'] = (time.time() - start) * 1000

    if filter_row is None:
        warn(f'Filter ID {filter_id} not found in CourseFilters.')
        return response

    nub_times = [row[1] for row in nub_records]
    pro_times = [row[1] for row in pro_records]
    nub_tier = filter_row[0]
    pro_tier = filter_row[1]

    '''
    There are 3 possible cases:
    1. Less than 50 nub times (and therefore less than 50 pro times as well)
       -> do not fit distribution, use sigmoid function
    2. 50 or more nub times but less than 50 pro times
       -> fit nub distribution, use sigmoid for pro
    3. 50 or more nub times and 50 or more pro times
       -> fit both distributions

    Overall/nub portion only depends on its own distribution.
    Pro portion takes the higher of the two distributions
    ((un)fitted nub or (un)fitted pro) to avoid situations where pro portion
    is lower than nub portion.

    '''
    if len(nub_times) >= 50:
        start = time.time()
        nub_dist, nub_params = refit_dist(nub_times, prev_nub_params)
        response['timings']['nub_fit_ms'] = (time.time() - start) * 1000
    elif len(nub_times) > 0:
        nub_dist, nub_params = None, (0,0,0,0,0)
        response['timings']['nub_fit_ms'] = 0
    else:
        warn(f'No overall records found for filter ID {filter_id}.')
        return response

    # Compute nub fractions
    start = time.time()
    nub_times_array = np.array(nub_times)
    new_fractions = common.get_dist_points_portion(
        nub_times_array,
        nub_times[0],
        nub_dist,
        nub_tier,
        nub_params[4],
        len(nub_times)
    )
    nub_records = [
        (record_id, time, fraction)
        for (record_id, time, _), fraction
        in zip(nub_records, new_fractions)
    ]
    response['timings']['nub_compute_ms'] = (time.time() - start) * 1000

    # Fit pro distribution
    if len(pro_times) >= 50:
        start = time.time()
        pro_dist, pro_params = refit_dist(pro_times, prev_pro_params)
        response['timings']['pro_fit_ms'] = (time.time() - start) * 1000
    elif len(pro_times) > 0:
        pro_dist, pro_params = None, (0,0,0,0,0)

    # Compute pro fractions if there are any pro records
    if len(pro_times) > 0:
        start = time.time()
        pro_times_array = np.array(pro_times)
        new_fractions = np.maximum(
            common.get_dist_points_portion(
                pro_times_array,
                pro_times[0],
                pro_dist,
                pro_tier,
                pro_params[4],
                len(pro_times)
            ),
            common.get_dist_points_portion(
                pro_times_array,
                nub_times[0],
                nub_dist,
                nub_tier,
                nub_params[4],
                len(nub_times)
            )
        )
        pro_records = [
            (record_id, time, fraction)
            for (record_id, time, _), fraction
            in zip(pro_records, new_fractions)
        ]
        response['timings']['pro_compute_ms'] = (time.time() - start) * 1000

    # Database write timing
    start = time.time()
    if len(nub_records) > 0:
        cursor.executemany("""
            UPDATE BestNubRecords SET
                points = ?
            WHERE
                record_id = ?
        """, [
            (points, record_id)
            for record_id, time, points
            in nub_records
        ])
    if len(pro_records) > 0:
        cursor.executemany("""
            UPDATE BestProRecords SET
                points = ?
            WHERE
                record_id = ?
        """, [
            (points, record_id)
            for record_id, time, points
            in pro_records
        ])
    if len(nub_times) >= 50:
        cursor.execute("""
            INSERT INTO PointDistributionData (
                filter_id,
                is_pro_leaderboard,
                a,
                b,
                loc,
                scale,
                top_scale
            ) VALUES (
                ?,
                0,
                ?,
                ?,
                ?,
                ?,
                ?
            )
            ON DUPLICATE KEY UPDATE
                a = VALUES(a),
                b = VALUES(b),
                loc = VALUES(loc),
                scale = VALUES(scale),
                top_scale = VALUES(top_scale)
            """, (
                filter_id,
                nub_params[0],
                nub_params[1],
                nub_params[2],
                nub_params[3],
                nub_params[4]
            ))
    if len(pro_times) >= 50:
        cursor.execute("""
            INSERT INTO PointDistributionData (
                filter_id,
                is_pro_leaderboard,
                a,
                b,
                loc,
                scale,
                top_scale
            ) VALUES (
                ?,
                1,
                ?,
                ?,
                ?,
                ?,
                ?
            )
            ON DUPLICATE KEY UPDATE
                a = VALUES(a),
                b = VALUES(b),
                loc = VALUES(loc),
                scale = VALUES(scale),
                top_scale = VALUES(top_scale)
            """, (
                filter_id,
                pro_params[0],
                pro_params[1],
                pro_params[2],
                pro_params[3],
                pro_params[4]
            ))
    database_conn.commit()
    response['timings']['db_write_ms'] = (time.time() - start) * 1000
    response['timings']['total_ms'] = sum(response['timings'].values())
    return response

def refit_dist(times, prev_params=None):
    if prev_params is not None:
        # Use previous parameters as initial guess for faster convergence
        a_init, b_init, loc_init, scale_init, _ = prev_params
        norminvgauss_params = stats.norminvgauss.fit(
            times,
            a_init, b_init,
            loc=loc_init,
            scale=scale_init
        )
    else:
        # Cold start - no initial parameters
        norminvgauss_params = stats.norminvgauss.fit(times)

    norminvgauss_dist = stats.norminvgauss(*norminvgauss_params)
    top_scale = norminvgauss_dist.sf(times[0])
    # Sanity safeguard
    if top_scale <= 0:
        warn('Fitted top_scale <= 0, resetting to 1')
        top_scale = 1

    a, b, loc, scale = norminvgauss_params
    return norminvgauss_dist, (a, b, loc, scale, top_scale)

if __name__ == '__main__':
    database_conn = None

    try:
        database_conn = open_database_conn()
    except mariadb.Error as e:
        sys.stderr.write(f'Error connecting to database: {e}\n')
        sys.stderr.write(traceback.format_exc() + '\n')
        sys.exit(1)

    for line in sys.stdin:
        try:
            response = process_input(database_conn, line)
            sys.stderr.flush()
            sys.stdout.write(json.dumps(response) + '\n')
            sys.stdout.flush()
        except KeyError as e:
            sys.stderr.write(f'Missing key in input data: {e}\n')
            sys.stderr.write(traceback.format_exc() + '\n')
            sys.exit(1)
        except json.JSONDecodeError as e:
            sys.stderr.write(f'JSON decode error: {e}\n')
            sys.stderr.write(traceback.format_exc() + '\n')
            sys.exit(1)
        except mariadb.Error as e:
            sys.stderr.write(f'Database error: {e}\n')
            sys.stderr.write(traceback.format_exc() + '\n')
            sys.exit(1)
        except Exception as e:
            sys.stderr.write(f'An unexpected error occurred: {e}\n')
            sys.stderr.write(traceback.format_exc() + '\n')
            sys.exit(1)

    try:
        database_conn.close()
    except mariadb.Error as e:
        sys.stderr.write(f'Failed to close database connection: {e}\n')
        sys.stderr.write(traceback.format_exc() + '\n')
