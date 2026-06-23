# Documentation example code lives under docs_src/; it is illustrative, not
# production source, so it should not be flagged for a missing test file.
from fastapi import FastAPI

app = FastAPI()


@app.get("/")
def read_root():
    return {"hello": "world"}
