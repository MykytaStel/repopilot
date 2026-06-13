# Five nested control-flow blocks: for > for > if > while > if. The innermost
# block sits at depth 5, one past the default limit of 4, so it is flagged.
def walk(matrix):
    for row in matrix:
        for cell in row:
            if cell is not None:
                while cell > 0:
                    if cell % 2 == 0:
                        cell -= 1
    return matrix
