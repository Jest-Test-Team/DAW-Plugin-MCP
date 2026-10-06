# FL Studio piano-roll script. Routing/mix graph is unavailable.

import flpianoroll as flp


def apply(form):
    score = flp.score
    notes = []
    for n in range(score.noteCount):
        note = score.getNote(n)
        notes.append({"pitch": note.number, "time": note.time, "length": note.length})
    form.dumpStatus("daw-mcp notes=%d" % len(notes))
