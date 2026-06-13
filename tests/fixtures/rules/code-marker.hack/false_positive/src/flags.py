# Feature-flag keys that contain the term are data, not review markers.
FEATURE_FLAGS = {"HACK": False, "BETA": True}
hack_mode = FEATURE_FLAGS["HACK"]
