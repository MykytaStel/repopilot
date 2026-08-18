# `from <package> import name` may import a submodule or a package member, so
# the derived `.get_document_model` candidate is not proof of a missing module.
from . import get_document_model

model = get_document_model()
