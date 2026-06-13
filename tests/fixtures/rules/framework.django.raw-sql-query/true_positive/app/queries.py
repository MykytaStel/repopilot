def get_user(cursor, user_id):
    # The user-supplied id is interpolated straight into the SQL string.
    cursor.execute(f"SELECT * FROM users WHERE id = {user_id}")
    return cursor.fetchone()
