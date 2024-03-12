def metadata():
    return {
        'sink': {
            'name': 'application/json',
        },
        'src': {
            'name': 'video/x-raw',
        },
    }


def tick(messages):
    return [
        type(messages[0])(
            payloads=[],
            # payloads=messages[0].payloads,
            value={
                'value': 'hello world',
            },
        ),
    ]
