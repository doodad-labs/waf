package fingerprint

import (
	"fmt"
	"net/http"
	"time"

	"github.com/doodad-labs/waf/pkg/metadata"
)

type FingerprintFunc func(*metadata.Metadata) (string, error)

// FingerprintHeaderInjector implements reverseproxy.HeaderInjector
type FingerprintHeaderInjector struct {
	HeaderName                       string
	FingerprintFunc                  FingerprintFunc
}

func NewFingerprintHeaderInjector(headerName string, fingerprintFunc FingerprintFunc) *FingerprintHeaderInjector {
	i := &FingerprintHeaderInjector{
		HeaderName:      headerName,
		FingerprintFunc: fingerprintFunc,
	}

	return i
}

func (i *FingerprintHeaderInjector) GetHeaderName() string {
	return i.HeaderName
}

func (i *FingerprintHeaderInjector) GetHeaderValue(req *http.Request) (string, error) {
	data, ok := metadata.FromContext(req.Context())
	if !ok {
		return "", fmt.Errorf("failed to get context")
	}

	start := time.Now()
	fp, err := i.FingerprintFunc(data)
	duration := time.Since(start)
	vlogf("fingerprint duration: %s", duration)

	return fp, err
}
