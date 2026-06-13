def get_user(cursor, user_id):
    # Parameterized query: the driver escapes the value, so this is safe.
    cursor.execute("SELECT * FROM users WHERE id = %s", [user_id])
    return cursor.fetchone()
