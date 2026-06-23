# Django AppConfig registration boilerplate: configuration/wiring, not behaviour
# that warrants a dedicated unit test.
from django.apps import AppConfig


class MyAppConfig(AppConfig):
    default_auto_field = "django.db.models.BigAutoField"
    name = "myapp"
