import { ComponentFixture, TestBed } from '@angular/core/testing';

import { ParagraphComponent } from './paragraph.component';

describe('ParagraphComponent', () => {
  let component: ParagraphComponent;
  let fixture: ComponentFixture<ParagraphComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ParagraphComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(ParagraphComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
