package kv

import (
	"github.com/pkg/errors"
	ethpb "github.com/prysmaticlabs/prysm/v4/proto/prysm/v1alpha1"
)

// SaveForkchoiceAttestation saves an forkchoice attestation in cache.
func (c *AttCaches) SaveForkchoiceAttestation(att *ethpb.Attestation) error {
	if att == nil {
		return nil
	}

	/*
		r, err := hashFn(att)
		if err != nil {
			return errors.Wrap(err, "could not tree hash attestation")
		}
	*/

	att = ethpb.CopyAttestation(att)
	c.forkchoiceAttLock.Lock()
	defer c.forkchoiceAttLock.Unlock()
	//c.forkchoiceAtt[r] = att
	c.forkchoiceAtt = append(c.forkchoiceAtt, att)

	return nil
}

// SaveForkchoiceAttestations saves a list of forkchoice attestations in cache.
func (c *AttCaches) SaveForkchoiceAttestations(atts []*ethpb.Attestation) error {
	for _, att := range atts {
		if err := c.SaveForkchoiceAttestation(att); err != nil {
			return err
		}
	}

	return nil
}

// ForkchoiceAttestations returns the forkchoice attestations in cache.
func (c *AttCaches) ForkchoiceAttestations() []*ethpb.Attestation {
	c.forkchoiceAttLock.RLock()
	defer c.forkchoiceAttLock.RUnlock()

	return c.forkchoiceAtt

	//atts := make([]*ethpb.Attestation, 0, len(c.forkchoiceAtt))
	//for _, att := range c.forkchoiceAtt {
	//	atts = append(atts, ethpb.CopyAttestation(att) /* Copied */)
	//}

	//return atts
}

// DeleteForkchoiceAttestation deletes a forkchoice attestation in cache.
func (c *AttCaches) DeleteForkchoiceAttestation(att *ethpb.Attestation) error {
	if att == nil {
		return nil
	}

	r1, err := hashFn(att)
	if err != nil {
		return errors.Wrap(err, "could not tree hash attestation")
	}

	index := -1
	for i, v := range c.forkchoiceAtt {
		r2, err := hashFn(v)
		if err != nil {
			return errors.Wrap(err, "could not tree hash attestation")
		}
		if r1 == r2 {
			index = i
			break
		}
	}

	if index != -1 {
		c.forkchoiceAtt = append(c.forkchoiceAtt[:index], (c.forkchoiceAtt)[index+1:]...)
	}

	/*
		r, err := hashFn(att)
		if err != nil {
			return errors.Wrap(err, "could not tree hash attestation")
		}

		c.forkchoiceAttLock.Lock()
		defer c.forkchoiceAttLock.Unlock()
		delete(c.forkchoiceAtt, r)
	*/

	return nil
}

// ForkchoiceAttestationCount returns the number of fork choice attestations key in the pool.
func (c *AttCaches) ForkchoiceAttestationCount() int {
	c.forkchoiceAttLock.RLock()
	defer c.forkchoiceAttLock.RUnlock()
	return len(c.forkchoiceAtt)
}
