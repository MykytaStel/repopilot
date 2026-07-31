The route reads an untrusted query parameter, but binds it separately from the SQL text. The taint source is real; the parameterized sink boundary must remain quiet.
