import { ComponentFixture, TestBed } from '@angular/core/testing';

import { AnchorComponent } from './anchor.component';

describe('AnchorComponent', () => {
  let component: AnchorComponent;
  let fixture: ComponentFixture<AnchorComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [AnchorComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(AnchorComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
