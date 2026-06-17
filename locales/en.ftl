language-name = English
language-menu = Language
app-menu-file = File
app-menu-view = View
app-menu-new-person = New person
app-menu-export-db = Export DB
app-menu-export-pdf = Export to PDF
app-menu-person-directory = Person directory

unsaved-changes = Unsaved changes
unknown-window = Unknown window
person-directory-title = Person Directory
new-person-title = New Person
person-details-title = Person Details
cemetery-library-title = Requiescat - Cemetery Library

library-empty = Your library is empty
library-count =
    { $count ->
        [one] 1 cemetery in your library
       *[other] { $count } cemeteries in your library
    }
brand-tagline = Cemetery records,
    carefully preserved.
brand-description = Manage maps and records from one secure library.
setup-library = Set up your library
setup-library-description = Create a new cemetery or import an existing one.
welcome-back = Welcome back
welcome-back-description = Choose where you would like to continue.
create-new-cemetery = Create new cemetery
import-cemetery = Import cemetery
open-cemetery = Open cemetery
export-cemetery = Export cemetery
export-named-cemetery = Export { $name }
create-cemetery = Create cemetery
create-cemetery-description = Enter a name for the cemetery.
cemetery-name = Cemetery name
back-to-menu = Back to menu
cemetery-library = Cemetery library
choose-cemetery = Choose a cemetery to open
no-cemeteries = No cemeteries yet
no-cemeteries-description = Import a cemetery database to add it to your library.
sqlite-cemetery = SQLite cemetery
open = Open

person = Person
person-not-found = Person not found
will-add-to-grave = Will be added to grave { $grave }
will-create-unassigned = Will be created unassigned
add-person = Add person
first-name = First name
last-name = Last name
date-of-birth-example = Date of birth, e.g. 30-04-1996
date-of-decease-example = Date of decease, e.g. 30-04-1996
date-of-birth = Date of birth
date-of-decease = Date of decease
grave = Grave { $grave }
grave-canvas = grave { $grave }
no-persons-associated = No persons associated yet
persons = Persons
search-people = Search names or dates
go-to-grave = Go to grave
unassign = Unassign
assign = Assign
born = Born { $date }

file-filter-sqlite-cemetery = SQLite cemetery
file-filter-pdf = PDF document
pdf-export-subtitle = A0 landscape printable cemetery map
empty-pdf-map = No graves to export
pdf-export-footer =
    { $count ->
        [one] 1 grave
       *[other] { $count } graves
    }
could-not-load-cemetery = Could not load cemetery: { $error }
library-unavailable = The cemetery library is unavailable.
cemetery-imported = Cemetery imported.
could-not-import-cemetery = Could not import cemetery: { $error }
could-not-create-cemetery = Could not create cemetery: { $error }
export-save-failed = Export cancelled because the cemetery could not be saved.
cemetery-exported = Cemetery exported.
could-not-export-cemetery = Could not export cemetery: { $error }
cemetery-pdf-exported = Cemetery PDF exported.
could-not-export-cemetery-pdf = Could not export cemetery PDF: { $error }
could-not-refresh-cemeteries = Could not refresh cemeteries: { $error }
save-failed = Save failed: { $error }
