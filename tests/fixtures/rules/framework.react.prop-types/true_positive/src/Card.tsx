import React from 'react';
import PropTypes from 'prop-types';

export function Card({ title }) {
  return <div>{title}</div>;
}

Card.propTypes = {
  title: PropTypes.string,
};
