import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';

import { Page } from '..';

describe('page.vue', () => {
  it('does not render title when passed', () => {
    const wrapper = mount(Page, {
      props: {
        title: 'Test Title',
      },
    });

    expect(wrapper.text()).not.toContain('Test Title');
  });

  it('does not render description when passed', () => {
    const wrapper = mount(Page, {
      props: {
        description: 'Test Description',
      },
    });

    expect(wrapper.text()).not.toContain('Test Description');
  });

  it('renders default slot content', () => {
    const wrapper = mount(Page, {
      slots: {
        default: '<p>Default Slot Content</p>',
      },
    });

    expect(wrapper.html()).toContain('<p>Default Slot Content</p>');
  });

  it('renders footer slot when showFooter is true', () => {
    const wrapper = mount(Page, {
      props: {
        showFooter: true,
      },
      slots: {
        footer: '<p>Footer Slot Content</p>',
      },
    });

    expect(wrapper.html()).toContain('<p>Footer Slot Content</p>');
  });

  it('applies the custom contentClass', () => {
    const wrapper = mount(Page, {
      props: {
        contentClass: 'custom-class',
      },
    });

    const contentDiv = wrapper.find('.p-4');
    expect(contentDiv.classes()).toContain('custom-class');
  });

  it('does not render title slot', () => {
    const wrapper = mount(Page, {
      props: {
        title: 'Test Title',
      },
      slots: {
        title: '<p>Title Slot Content</p>',
      },
    });

    expect(wrapper.text()).not.toContain('Title Slot Content');
    expect(wrapper.html()).not.toContain('Test Title');
  });

  it('does not render description slot', () => {
    const wrapper = mount(Page, {
      props: {
        description: 'Test Description',
      },
      slots: {
        description: '<p>Description Slot Content</p>',
      },
    });

    expect(wrapper.text()).not.toContain('Description Slot Content');
    expect(wrapper.html()).not.toContain('Test Description');
  });
});
