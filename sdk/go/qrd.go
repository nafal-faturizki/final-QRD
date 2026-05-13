package qrd

// FileReader is a Go-side QRD SDK placeholder.
type FileReader struct {
	Path      string
	MasterKey []byte
}

// FileWriter is a Go-side QRD SDK placeholder.
type FileWriter struct {
	Path   string
	Schema map[string]any
}

// InspectHeader is a placeholder that will later call the Rust core through CGO.
func InspectHeader(path string) (map[string]any, error) {
	_ = path
	return nil, errNotImplemented{}
}

// InspectFooter is a placeholder that will later call the Rust core through CGO.
func InspectFooter(path string) (map[string]any, error) {
	_ = path
	return nil, errNotImplemented{}
}

// WriteRow is a placeholder that will later call the Rust core through CGO.
func (w *FileWriter) WriteRow(row map[string]any) error {
	_ = row
	return errNotImplemented{}
}

// Finish is a placeholder that will later call the Rust core through CGO.
func (w *FileWriter) Finish() error {
	return errNotImplemented{}
}

type errNotImplemented struct{}

func (errNotImplemented) Error() string { return "CGO binding not added yet" }
