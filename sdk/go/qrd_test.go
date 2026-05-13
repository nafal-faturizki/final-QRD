package qrd

import "testing"

func TestPlaceholdersReturnErrors(t *testing.T) {
	if _, err := InspectHeader("example.qrd"); err == nil {
		t.Fatal("expected error")
	}
	if _, err := InspectFooter("example.qrd"); err == nil {
		t.Fatal("expected error")
	}
}
