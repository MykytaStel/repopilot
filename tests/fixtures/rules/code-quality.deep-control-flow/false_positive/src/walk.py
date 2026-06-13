# Four nested control-flow blocks: for > for > if > while. This sits exactly at
# the default limit of 4, so it is not flagged — the rule fires only past it.
def walk(matrix):
    for row in matrix:
        for cell in row:
            if cell is not None:
                while cell > 0:
                    cell -= 1
    return matrix
